#!/bin/sh
# Restores an encrypted backup. Needs an age private key, which never
# lives on the server: mount it read-only for this one command.
#
#   docker compose stop finbot
#   docker compose run --rm -v "$HOME/finbot-age.key:/key:ro" backup \
#       restore.sh /backups/daily/finbot-2026-09-21T0300.dump.age /key
#   docker compose start finbot
set -eu
# shellcheck source=lib.sh
. /usr/local/lib/finbot-backup.sh

[ "$#" -eq 2 ] || fail "usage: restore.sh <backup.dump.age> <age-identity-file>"
[ -r "$1" ] || fail "cannot read backup file $1"
[ -r "$2" ] || fail "cannot read age identity file $2"
require_env PGHOST PGUSER PGDATABASE
load_database_password

work="$(mktemp -d "${TMPDIR:-/tmp}/finbot-restore.XXXXXX")"
trap 'rm -rf "$work"' EXIT
age --decrypt --identity "$2" --output "$work/finbot.dump" "$1" || fail "decryption failed: wrong key or corrupted file"
pg_restore --clean --if-exists --no-owner --single-transaction --dbname="$PGDATABASE" "$work/finbot.dump" \
    || fail "pg_restore failed; the database was left unchanged"
log INFO "restored $(basename "$1") into $PGDATABASE"
