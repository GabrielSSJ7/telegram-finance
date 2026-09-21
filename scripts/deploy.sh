#!/bin/sh
# Deploys a release on the VPS. Run from the checkout (e.g. /opt/finbot):
#
#   ./scripts/deploy.sh v0.4.0
#
# Steps: check out the tag (compose files and Caddyfile), back up the
# database before migrations run, pull images, restart, wait for healthy.
set -eu
cd "$(dirname "$0")/.."

tag="${1:?usage: deploy.sh <git tag, e.g. v0.4.0>}"

# With compose.proxy.yml, another stack's nginx (PROXY_CONTAINER in .env)
# serves the API. It loses the finbot-proxy network whenever that stack
# recreates it, so every deploy re-attaches it; connecting is live and
# does not restart the other nginx.
attach_shared_proxy() {
    proxy="$(sed -n 's/^PROXY_CONTAINER=//p' .env | tail -n 1)"
    [ -n "$proxy" ] || return 0
    attached="$(docker network inspect finbot-proxy --format '{{range .Containers}}{{.Name}} {{end}}')"
    case " $attached " in
        *" $proxy "*) ;;
        *) docker network connect finbot-proxy "$proxy" && echo "attached $proxy to finbot-proxy" ;;
    esac
}

git fetch --tags --quiet
git checkout --quiet "$tag"

if docker compose ps --status running --services 2>/dev/null | grep -qx backup; then
    docker compose exec -T backup backup.sh pre-deploy || echo "warning: pre-deploy backup failed" >&2
fi

# Record the running tag so a plain `docker compose up` after a reboot
# starts the same version.
touch .env
grep -v '^FINBOT_TAG=' .env > .env.next || true
echo "FINBOT_TAG=$tag" >> .env.next
mv .env.next .env

docker compose pull --quiet
docker compose up -d --remove-orphans
attach_shared_proxy

container="$(docker compose ps -q finbot)"
for attempt in $(seq 1 30); do
    status="$(docker inspect --format '{{.State.Health.Status}}' "$container")"
    if [ "$status" = healthy ]; then
        echo "finbot $tag is healthy"
        docker image prune -f > /dev/null
        exit 0
    fi
    echo "waiting for finbot ($status, attempt $attempt/30)"
    sleep 5
done
echo "finbot $tag did not become healthy; check: docker compose logs finbot" >&2
exit 1
