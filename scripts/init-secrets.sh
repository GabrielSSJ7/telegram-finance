#!/bin/sh
# Creates ./secrets for compose.yml: a random Postgres password, the
# matching DATABASE_URL, and the Telegram bot token (read from stdin).
#
#   ./scripts/init-secrets.sh            # prompts for the bot token
#   printf '%s' "$TOKEN" | ./scripts/init-secrets.sh
#
# Files are 0444 inside a 0700 directory: the finbot container runs as a
# non-root user and must read them, while other host users cannot reach
# the directory.
set -eu
cd "$(dirname "$0")/.."

if [ -e secrets/postgres_password ]; then
    echo "secrets/ already exists; delete it first to regenerate" >&2
    exit 1
fi
mkdir -p secrets
chmod 0700 secrets

password="$(head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n')"
printf '%s' "$password" > secrets/postgres_password
printf 'postgres://finbot:%s@postgres:5432/finbot' "$password" > secrets/database_url

if [ -t 0 ]; then
    printf 'Telegram bot token (from @BotFather): ' >&2
    stty -echo; read -r token; stty echo; echo >&2
else
    read -r token || true
fi
[ -n "${token:-}" ] || { echo "empty bot token" >&2; exit 1; }
printf '%s' "$token" > secrets/telegram_bot_token

chmod 0444 secrets/*
echo "secrets written to $(pwd)/secrets" >&2
