# Deploying on a Hostinger VPS

Docker needs a **VPS (KVM)** plan. Hostinger's "Cloud Hosting" plans are
shared hosting and cannot run containers. KVM 1 (1 vCPU, 4 GB RAM) is
enough: the VPS never compiles Rust, it pulls images built by GitHub
Actions.

If another stack on the VPS already owns ports 80 and 443 with its own
nginx, follow [deploy-shared-nginx.md](deploy-shared-nginx.md) instead of
the Caddy setup below.

## What runs

| Service | Image | Reachable from |
|---|---|---|
| `caddy` | `caddy:2-alpine` | the internet, ports 80 and 443 only |
| `finbot` | `ghcr.io/gabrielssj7/finbot` | Caddy; calls out to Telegram |
| `postgres` | `postgres:18-alpine` | finbot and backup only (internal network) |
| `backup` | `ghcr.io/gabrielssj7/finbot-backup` | Postgres; calls out to Telegram |

## One-time setup

1. **VPS.** Pick Ubuntu 24.04 with Docker in hPanel (or install Docker
   Engine and the compose plugin). Log in as a non-root user in the
   `docker` group.
2. **Firewall.** Docker bypasses `ufw` for published ports, which is why
   only Caddy publishes any. Still close everything else:
   ```sh
   sudo ufw default deny incoming
   sudo ufw allow 22/tcp && sudo ufw allow 80/tcp && sudo ufw allow 443
   sudo ufw enable
   ```
3. **DNS.** Create an `A` record `api.<your-domain>` pointing at the VPS IP.
4. **Code.** Clone the repository (a read-only deploy key is enough):
   ```sh
   sudo mkdir -p /opt/finbot && sudo chown "$USER" /opt/finbot
   git clone git@github.com:GabrielSSJ7/telegram-finance.git /opt/finbot
   ```
5. **Registry.** If the GHCR packages are private, create a GitHub token
   with only `read:packages` and run `docker login ghcr.io`.
6. **Secrets.** `./scripts/init-secrets.sh` creates `secrets/` with a random
   database password and asks for the Telegram bot token
   ([setup-telegram.md](setup-telegram.md)).
7. **Backup keys.** Each spouse runs `age-keygen -o finbot-age.key` on their
   own computer and keeps the file in a password manager. Only the public
   keys (`age1...`) go to the server. **Losing both private keys makes every
   backup unreadable.**
8. **`.env`** in `/opt/finbot`:
   ```sh
   API_DOMAIN=api.example.com
   ACME_EMAIL=you@example.com
   ALLOWED_TELEGRAM_USER_IDS=111111111,222222222
   AGE_RECIPIENTS=age1...,age1...
   HOUSEHOLD_TIMEZONE=America/Sao_Paulo
   ```
9. **First start.** `./scripts/deploy.sh v0.4.0` (use the newest tag).
10. **API key** for REST clients:
    ```sh
    docker compose exec finbot finbot api-key create dashboard
    ```
    The token is printed once.

## Releasing

Push a tag; GitHub Actions builds and pushes both images:

```sh
git tag v0.4.1 && git push origin v0.4.1
```

Then on the VPS: `./scripts/deploy.sh v0.4.1`. It checks out the tag, backs
up the database, pulls, restarts, and waits for the health check.

To deploy automatically, set the repository variable `AUTO_DEPLOY=true` and
the secrets `VPS_HOST`, `VPS_USER`, `VPS_SSH_KEY` (a key allowed only to run
the deploy). The `release` workflow then runs `deploy.sh` over SSH.

## Day to day

```sh
docker compose ps                    # health of every service
docker compose logs -f finbot        # JSON logs
curl https://api.example.com/healthz # database and Telegram checks
```

Logs rotate (10 MB × 5 files per service).
