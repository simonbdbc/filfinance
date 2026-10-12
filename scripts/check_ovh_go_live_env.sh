#!/usr/bin/env bash
set -euo pipefail

# This is intentionally separate from the server boot checks: a new VPS may
# start locally before all paid/third-party integrations are ready. The DNS
# cutover must not occur until this gate passes.
ENV_FILE="${1:-.env.server.prod}"

fail() {
  echo "OVH go-live preflight failed: $*" >&2
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

require_real_value() {
  local name="$1"
  local value

  value="$(env_value "$name")"
  case "$value" in
    '' | *replace-with* | *your-* | *example.com* | *example.net* | *CHANGE-ME*)
      fail "$name must be set to a non-placeholder value"
      ;;
  esac
}

require_equal() {
  local name="$1"
  local expected="$2"

  [ "$(env_value "$name")" = "$expected" ] \
    || fail "$name must equal $expected"
}

[ -r "$ENV_FILE" ] || fail "$ENV_FILE is not readable by $(id -un)"

require_equal FILFINANCE_ENV_FILE .env.server.prod
require_equal FILFINANCE_AUTH_TRAEFIK_ENABLED false
require_equal FILFINANCE_TRAEFIK_ENABLED false
require_equal NEWS_FINANCIAL_JUICE_ENABLED true
require_equal NEWS_FINANCIAL_JUICE_BROWSER_ENABLED true
require_equal NEWS_FINANCIAL_JUICE_BROWSER_HEADLESS false

require_equal SMTP_HOST smtp-relay.brevo.com
require_equal SMTP_PORT 587
require_equal SMTP_SECURE false
require_equal SMTP_REQUIRE_TLS true
require_real_value SMTP_USER
require_real_value SMTP_PASSWORD
require_equal SMTP_FROM_EMAIL noreply@filfinance.fr
require_real_value SMTP_FROM_NAME
require_real_value NEWS_FINANCIAL_JUICE_EMAIL
require_real_value NEWS_FINANCIAL_JUICE_PASSWORD
require_real_value NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN
require_real_value OPENROUTER_API_KEY

echo 'OVH go-live preflight passed'
