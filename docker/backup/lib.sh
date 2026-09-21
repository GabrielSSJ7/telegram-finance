# Shared helpers for backup.sh and restore.sh (POSIX sh).

log() {
    printf '{"timestamp":"%s","level":"%s","message":"%s","target":"finbot::backup"}\n' \
        "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$1" "$2"
}

fail() {
    log ERROR "$1"
    exit 1
}

require_env() {
    for name in "$@"; do
        eval "value=\${$name:-}"
        [ -n "$value" ] || fail "missing environment variable $name"
    done
}

# Exports PGPASSWORD from the Docker secret so psql/pg_dump can connect.
load_database_password() {
    [ -r "${POSTGRES_PASSWORD_FILE:-}" ] || fail "cannot read POSTGRES_PASSWORD_FILE=${POSTGRES_PASSWORD_FILE:-unset}"
    PGPASSWORD="$(cat "$POSTGRES_PASSWORD_FILE")"
    export PGPASSWORD
}

telegram_token() {
    [ -r "${TELEGRAM_BOT_TOKEN_FILE:-}" ] || return 1
    cat "$TELEGRAM_BOT_TOKEN_FILE"
}

# Private chats of members who opted in to receive backups.
backup_chats() {
    psql -Atc "select dm_chat_id from members where receives_backups and dm_chat_id is not null"
}
