#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-${FILFINANCE_ENV_FILE:-.env.server}}"
TARGET="${2:-prod}"

fail() {
  echo "Deploy environment preflight failed: $*" >&2
  exit 1
}

env_value() {
  local name="$1"
  local value

  value="$(grep -E "^${name}=" "$ENV_FILE" | tail -n 1 | cut -d= -f2- | tr -d '\r' || true)"
  value="${value#\"}"
  value="${value%\"}"
  value="${value#\'}"
  value="${value%\'}"
  printf '%s' "$value"
}

require_equal() {
  local name="$1"
  local expected="$2"
  local actual

  actual="$(env_value "$name")"
  [ "$actual" = "$expected" ] || fail "$name must equal $expected for $TARGET"
}

[ -r "$ENV_FILE" ] || fail "$ENV_FILE is not readable by $(id -un)"
case "$TARGET" in
  prod)
    suffix="prod"
    api_host="api.filfinance.fr"
    backend_port="8088"
    edge_publish_url="https://live.filfinance.fr/publish/live"
    ;;
  dev)
    suffix="dev"
    api_host="api.dev.filfinance.fr"
    backend_port="18088"
    origin_host="origin.dev.filfinance.fr"
    ;;
  *) fail "target must be dev or prod" ;;
esac

require_equal COMPOSE_PROJECT_NAME "filfinance_${suffix}"
require_equal FILFINANCE_ENV_FILE ".env.server.${suffix}"
require_equal FILFINANCE_SERVICE_CONTAINER_NAME "filfinance_service_${suffix}"
require_equal FILFINANCE_BROWSER_COLLECTOR_CONTAINER_NAME "financialjuice_browser_collector_${suffix}"
require_equal FILFINANCE_POSTGRES_CONTAINER_NAME "filfinance_postgres_${suffix}"
require_equal FILFINANCE_AUTH_CONTAINER_NAME "filfinance_auth_${suffix}"
require_equal FILFINANCE_GATEWAY_CONTAINER_NAME "filfinance_gateway_${suffix}"
require_equal FILFINANCE_INTERNAL_NETWORK_NAME "filfinance_internal_${suffix}"
require_equal FILFINANCE_BROWSER_DATA_VOLUME_NAME "financialjuice_browser_data_${suffix}"
require_equal FILFINANCE_POSTGRES_DATA_VOLUME_NAME "filfinance_postgres_data_${suffix}"
require_equal FILFINANCE_AUTH_TRAEFIK_ROUTER_NAME "filfinance-auth-${suffix}"
require_equal FILFINANCE_AUTH_TRAEFIK_SERVICE_NAME "filfinance-auth-${suffix}"
require_equal FILFINANCE_TRAEFIK_ROUTER_NAME "filfinance-api-${suffix}"
require_equal FILFINANCE_TRAEFIK_SERVICE_NAME "filfinance-service-${suffix}"
require_equal FILFINANCE_API_HOST "$api_host"
if [ "$TARGET" = "dev" ]; then
  require_equal FILFINANCE_ORIGIN_HOST "$origin_host"
  require_equal FILFINANCE_TRAEFIK_CERT_RESOLVER "bunny_dns"
  require_equal NEWS_EDGE_PUBLISH_ENABLED "false"
  require_equal NEWS_LIVE_FANOUT_ENABLED "true"
  require_equal NEWS_LIVE_REDIS_URL "redis://filfinance-redis:6379/0"
else
  require_equal FILFINANCE_TRAEFIK_CERT_RESOLVER "lets_encrypt"
  require_equal FILFINANCE_AUTH_TRAEFIK_ENABLED "false"
  require_equal FILFINANCE_TRAEFIK_ENABLED "false"
fi
require_equal FILFINANCE_BACKEND_HOST_BIND "127.0.0.1"
require_equal FILFINANCE_BACKEND_PORT "$backend_port"
require_equal FILFINANCE_SERVICE_DATA_DIR "./volumes/service/data"
require_equal NEWS_SERVER_ENV "production"
if [ "$TARGET" = "prod" ]; then
  require_equal NEWS_EDGE_PUBLISH_URL "$edge_publish_url"
fi

echo "Deploy environment preflight passed for $TARGET"
