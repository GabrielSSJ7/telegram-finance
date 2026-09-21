# Backup sidecar: pg_dump from the same major version as the server,
# age for encryption, curl for Telegram, busybox crond for the schedule.
FROM postgres:18-alpine
RUN apk add --no-cache age curl tzdata
COPY backup.sh restore.sh /usr/local/bin/
COPY lib.sh /usr/local/lib/finbot-backup.sh
COPY crontab /etc/crontabs/root
RUN chmod 0755 /usr/local/bin/backup.sh /usr/local/bin/restore.sh
# CMD, not ENTRYPOINT: `docker compose run backup restore.sh ...` replaces it.
CMD ["crond", "-f", "-l", "6"]
