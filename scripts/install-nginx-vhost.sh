#!/bin/sh
# Installs the finbot site into another stack's nginx without risking it:
# the new file is checked with `nginx -t` inside the running container and
# removed again if the check fails, so the other sites never see a broken
# config. Run on the VPS from the finbot checkout:
#
#   ./scripts/install-nginx-vhost.sh finbot.example.com \
#       /opt/peopleandquality/nginx/conf.d pq-nginx
#
# The certificate for the domain must exist first (see
# docs/deploy-shared-nginx.md): nginx refuses a site whose cert is missing.
set -eu
cd "$(dirname "$0")/.."

domain="${1:?usage: install-nginx-vhost.sh <domain> <conf.d dir> <nginx container>}"
conf_dir="${2:?missing conf.d directory}"
container="${3:?missing nginx container name}"
target="$conf_dir/finbot.conf"
# Host path of the certificates the nginx container mounts.
certs="${LETSENCRYPT_DIR:-/etc/letsencrypt}"
previous="$target.previous"

case "$domain" in
    *[!a-z0-9.-]* | "" | .* | *.) echo "invalid domain: $domain (expected e.g. finbot.example.com)" >&2; exit 1 ;;
esac
[ -f "$certs/live/$domain/fullchain.pem" ] || {
    echo "no certificate at $certs/live/$domain; issue it first" >&2
    exit 1
}

[ -f "$target" ] && cp "$target" "$previous"
sed "s/__DOMAIN__/$domain/g" docker/nginx/finbot.conf.template > "$target"

if ! docker exec "$container" nginx -t; then
    echo "nginx rejected the finbot site; restoring the previous state" >&2
    if [ -f "$previous" ]; then mv "$previous" "$target"; else rm -f "$target"; fi
    exit 1
fi
rm -f "$previous"
docker exec "$container" nginx -s reload
echo "finbot site live at https://$domain"
