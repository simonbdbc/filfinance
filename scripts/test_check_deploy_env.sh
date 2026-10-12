#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PREFLIGHT="$ROOT_DIR/scripts/check_deploy_env.sh"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

write_env() {
  local target="$1"
  local suffix api_host backend_port edge_url origin_host

  if [ "$target" = "prod" ]; then
    suffix="prod"
    api_host="api.filfinance.fr"
    backend_port="8088"
    edge_url="https://live.filfinance.fr/publish/live"
  else
    suffix="dev"
    api_host="api.dev.filfinance.fr"
    backend_port="18088"
    origin_host="origin.dev.filfinance.fr"
  fi

  printf '%s\n' \
    "COMPOSE_PROJECT_NAME=filfinance_${suffix}" \
    "FILFINANCE_ENV_FILE=.env.server.${suffix}" \
    "FILFINANCE_SERVICE_CONTAINER_NAME=filfinance_service_${suffix}" \
    "FILFINANCE_BROWSER_COLLECTOR_CONTAINER_NAME=financialjuice_browser_collector_${suffix}" \
    "FILFINANCE_POSTGRES_CONTAINER_NAME=filfinance_postgres_${suffix}" \
    "FILFINANCE_AUTH_CONTAINER_NAME=filfinance_auth_${suffix}" \
    "FILFINANCE_GATEWAY_CONTAINER_NAME=filfinance_gateway_${suffix}" \
    "FILFINANCE_INTERNAL_NETWORK_NAME=filfinance_internal_${suffix}" \
    "FILFINANCE_BROWSER_DATA_VOLUME_NAME=financialjuice_browser_data_${suffix}" \
    "FILFINANCE_POSTGRES_DATA_VOLUME_NAME=filfinance_postgres_data_${suffix}" \
    "FILFINANCE_AUTH_TRAEFIK_ROUTER_NAME=filfinance-auth-${suffix}" \
    "FILFINANCE_AUTH_TRAEFIK_SERVICE_NAME=filfinance-auth-${suffix}" \
    "FILFINANCE_TRAEFIK_ROUTER_NAME=filfinance-api-${suffix}" \
    "FILFINANCE_TRAEFIK_SERVICE_NAME=filfinance-service-${suffix}" \
    "FILFINANCE_TRAEFIK_CERT_RESOLVER=$([ "$target" = "dev" ] && printf '%s' bunny_dns || printf '%s' lets_encrypt)" \
    "FILFINANCE_API_HOST=${api_host}" \
    "FILFINANCE_BACKEND_HOST_BIND=127.0.0.1" \
    "FILFINANCE_BACKEND_PORT=${backend_port}" \
    "FILFINANCE_SERVICE_DATA_DIR=./volumes/service/data" \
    "NEWS_SERVER_ENV=production" \
    > "$TMP_DIR/$target.env"

  if [ "$target" = "dev" ]; then
    printf '%s\n' \
      "FILFINANCE_ORIGIN_HOST=${origin_host}" \
      "NEWS_EDGE_PUBLISH_ENABLED=false" \
      "NEWS_LIVE_FANOUT_ENABLED=true" \
      "NEWS_LIVE_REDIS_URL=redis://filfinance-redis:6379/0" \
      >> "$TMP_DIR/$target.env"
  else
    printf '%s\n' \
      "FILFINANCE_AUTH_TRAEFIK_ENABLED=false" \
      "FILFINANCE_TRAEFIK_ENABLED=false" \
      "NEWS_EDGE_PUBLISH_URL=${edge_url}" \
      >> "$TMP_DIR/$target.env"
  fi
}

expect_pass() {
  "$@" >/dev/null
}

expect_fail_contains() {
  local expected="$1"
  local output
  shift

  if output="$("$@" 2>&1)"; then
    echo "Expected command to fail: $*" >&2
    exit 1
  fi
  printf '%s' "$output" | grep -Fq -- "$expected" || {
    echo "Expected failure to contain: $expected" >&2
    echo "$output" >&2
    exit 1
  }
}

write_env prod
expect_pass bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod
write_env dev
expect_pass bash "$PREFLIGHT" "$TMP_DIR/dev.env" dev

write_env dev
sed -i.bak 's/^FILFINANCE_AUTH_CONTAINER_NAME=.*/FILFINANCE_AUTH_CONTAINER_NAME=filfinance_auth_prod/' "$TMP_DIR/dev.env"
expect_fail_contains "FILFINANCE_AUTH_CONTAINER_NAME must equal filfinance_auth_dev" bash "$PREFLIGHT" "$TMP_DIR/dev.env" dev

write_env dev
sed -i.bak 's/^FILFINANCE_INTERNAL_NETWORK_NAME=.*/FILFINANCE_INTERNAL_NETWORK_NAME=filfinance_internal_prod/' "$TMP_DIR/dev.env"
expect_fail_contains "FILFINANCE_INTERNAL_NETWORK_NAME must equal filfinance_internal_dev" bash "$PREFLIGHT" "$TMP_DIR/dev.env" dev

write_env dev
sed -i.bak 's/^FILFINANCE_BACKEND_PORT=.*/FILFINANCE_BACKEND_PORT=8088/' "$TMP_DIR/dev.env"
expect_fail_contains "FILFINANCE_BACKEND_PORT must equal 18088" bash "$PREFLIGHT" "$TMP_DIR/dev.env" dev

write_env dev
sed -i.bak 's/^FILFINANCE_TRAEFIK_CERT_RESOLVER=.*/FILFINANCE_TRAEFIK_CERT_RESOLVER=lets_encrypt/' "$TMP_DIR/dev.env"
expect_fail_contains "FILFINANCE_TRAEFIK_CERT_RESOLVER must equal bunny_dns" bash "$PREFLIGHT" "$TMP_DIR/dev.env" dev

write_env prod
sed -i.bak 's#^NEWS_EDGE_PUBLISH_URL=.*#NEWS_EDGE_PUBLISH_URL=https://live.dev.filfinance.fr/publish/live#' "$TMP_DIR/prod.env"
expect_fail_contains "NEWS_EDGE_PUBLISH_URL must equal https://live.filfinance.fr/publish/live" bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod

write_env prod
sed -i.bak 's/^FILFINANCE_AUTH_TRAEFIK_ENABLED=.*/FILFINANCE_AUTH_TRAEFIK_ENABLED=true/' "$TMP_DIR/prod.env"
expect_fail_contains "FILFINANCE_AUTH_TRAEFIK_ENABLED must equal false" bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod

echo "Deploy environment preflight tests passed"
