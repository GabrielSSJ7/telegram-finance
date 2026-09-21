#!/bin/sh
# Daily backup: pg_dump -> verify -> age-encrypt -> rotate -> Telegram DMs.
#
#   backup.sh scheduled   daily run from cron (keeps 7 daily, 4 weekly)
#   backup.sh pre-deploy  run by scripts/deploy.sh (keeps 5, not sent)
#   backup.sh verify      like scheduled, plus a full test restore
#
# The plaintext dump only exists in a temporary directory (tmpfs in
# compose) and is deleted on exit; only the encrypted file is kept.
set -eu
# shellcheck source=lib.sh
. /usr/local/lib/finbot-backup.sh

REASON="${1:-scheduled}"
BACKUP_ROOT="${BACKUP_ROOT:-/backups}"
MAX_TELEGRAM_BYTES=$((49 * 1024 * 1024))

main() {
    require_env PGHOST PGUSER PGDATABASE AGE_RECIPIENTS
    load_database_password
    work="$(mktemp -d "${TMPDIR:-/tmp}/finbot-backup.XXXXXX")"
    trap 'rm -rf "$work"' EXIT
    stamp="$(date +%Y-%m-%dT%H%M)"
    dump="$work/finbot.dump"
    pg_dump --format=custom --file="$dump" || fail "pg_dump failed"
    pg_restore --list "$dump" > /dev/null || fail "dump is not readable by pg_restore"
    if [ "$REASON" = verify ] || [ "$(date +%u)" = 7 ]; then test_restore "$dump"; fi
    encrypted="$work/finbot-$stamp.dump.age"
    encrypt "$dump" "$encrypted"
    store "$encrypted"
    record_success
    [ "$REASON" = pre-deploy ] || send_to_members "$encrypted"
    log INFO "backup $REASON finished: $(basename "$encrypted") ($(wc -c < "$encrypted") bytes)"
}

# Restores into a scratch database and checks the household row exists.
test_restore() {
    scratch="finbot_restore_check"
    dropdb --if-exists "$scratch"
    createdb --template=template0 "$scratch"
    pg_restore --no-owner --dbname="$scratch" "$1" || { dropdb "$scratch"; fail "test restore failed"; }
    rows="$(psql -d "$scratch" -Atc "select count(*) from household")"
    dropdb "$scratch"
    [ "$rows" = 1 ] || fail "test restore has $rows household rows, expected 1"
    log INFO "test restore succeeded"
}

encrypt() {
    recipients=""
    for key in $(printf '%s' "$AGE_RECIPIENTS" | tr ',' ' '); do recipients="$recipients -r $key"; done
    # shellcheck disable=SC2086 # recipients are intentionally split
    age $recipients -o "$2" "$1" || fail "age encryption failed (check AGE_RECIPIENTS)"
}

store() {
    case "$REASON" in
        pre-deploy) keep "$1" pre-deploy 5 ;;
        *)
            keep "$1" daily 7
            if [ "$(date +%u)" = 7 ]; then keep "$1" weekly 4; fi
            ;;
    esac
}

# Copies $1 into $BACKUP_ROOT/$2 and keeps only the newest $3 files there.
keep() {
    mkdir -p "$BACKUP_ROOT/$2"
    cp "$1" "$BACKUP_ROOT/$2/"
    # shellcheck disable=SC2012 # names are our own timestamps, no odd characters
    ls -1t "$BACKUP_ROOT/$2"/*.age | tail -n "+$(($3 + 1))" | xargs -r rm -f
}

# finbot's backup_watch job alerts when this is older than 26 hours.
record_success() {
    psql -q -c "insert into job_runs (job, run_date, status) values ('backup', current_date, 'succeeded')
                on conflict (job, run_date) do update set status = 'succeeded',
                attempts = job_runs.attempts + 1, updated_at = now()" \
        || log WARN "could not record backup success in job_runs"
}

send_to_members() {
    token="$(telegram_token)" || { log WARN "no TELEGRAM_BOT_TOKEN_FILE; backup not sent"; return 0; }
    size="$(wc -c < "$1")"
    [ "$size" -le "$MAX_TELEGRAM_BYTES" ] || { notify "⚠️ Backup de $size bytes passou do limite do Telegram; ficou só no servidor."; return 0; }
    for chat in $(backup_chats); do
        curl -sS --fail -o /dev/null -F "chat_id=$chat" -F "document=@$1" \
            -F "caption=🔐 Backup $(date +%d/%m/%Y) (criptografado com age)" \
            "https://api.telegram.org/bot$token/sendDocument" || log WARN "could not send backup to chat $chat"
    done
}

notify() {
    token="$(telegram_token)" || return 0
    for chat in $(backup_chats 2>/dev/null); do
        curl -sS -o /dev/null --data-urlencode "chat_id=$chat" --data-urlencode "text=$1" \
            "https://api.telegram.org/bot$token/sendMessage" || true
    done
}

# Not `if ! (main)`: POSIX turns off `set -e` inside an `if` condition,
# which would let a failed step go unnoticed.
set +e
(set -e; main)
status=$?
set -e
if [ "$status" -ne 0 ]; then
    notify "⚠️ O backup de hoje ($REASON) falhou. Veja os logs do container backup."
    exit 1
fi
