# Backups and restore

## What happens every night

At 03:00 (household timezone) the `backup` container:

1. dumps the database with `pg_dump` into a RAM-only temporary directory;
2. checks the dump with `pg_restore --list`, and on Sundays restores it
   into a scratch database to prove it can be restored;
3. encrypts it with [age](https://age-encryption.org) to both spouses'
   public keys (`AGE_RECIPIENTS`);
4. keeps 7 daily and 4 weekly encrypted copies in the `backups` volume;
5. sends the encrypted file to each spouse's private chat with the bot
   (after they sent `/start` there);
6. records the success, so finbot can warn when a backup is missing.

If any step fails, both of you get a Telegram message.

`scripts/deploy.sh` also takes a `pre-deploy` backup (last 5 kept, not
sent) before new migrations run.

## Run one now

```sh
docker compose exec backup backup.sh scheduled   # normal run
docker compose exec backup backup.sh verify      # plus a test restore
```

## Restore

The private key never lives on the server; bring it for this one command.

```sh
cd /opt/finbot
docker compose stop finbot
scp laptop:finbot-age.key /tmp/finbot-age.key
docker compose run --rm -v /tmp/finbot-age.key:/key:ro backup \
    restore.sh /backups/daily/finbot-2026-09-21T0300.dump.age /key
shred -u /tmp/finbot-age.key
docker compose start finbot
```

To restore a file you received in Telegram, copy it into the volume first:
`docker compose cp finbot-....dump.age backup:/backups/`.

`restore.sh` runs in a single transaction: if anything fails, the database
is left as it was.

## Test the decryption path

Every few months, decrypt a Telegram copy on your own computer:

```sh
age --decrypt -i finbot-age.key -o test.dump finbot-2026-09-21T0300.dump.age
pg_restore --list test.dump | head
```
