#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHECK_SCRIPT="$ROOT_DIR/scripts/check_ovh_go_live_env.sh"
fixture="$(mktemp)"
trap 'rm -f "$fixture"' EXIT

if bash "$CHECK_SCRIPT" "$ROOT_DIR/.env.server.example" >/dev/null 2>&1; then
  echo 'go-live preflight unexpectedly accepted the server template' >&2
  exit 1
fi

printf '%s\n' \
  'FILFINANCE_ENV_FILE=.env.server.prod' \
  'FILFINANCE_AUTH_TRAEFIK_ENABLED=false' \
  'FILFINANCE_TRAEFIK_ENABLED=false' \
  'NEWS_FINANCIAL_JUICE_ENABLED=true' \
  'NEWS_FINANCIAL_JUICE_BROWSER_ENABLED=true' \
  'NEWS_FINANCIAL_JUICE_BROWSER_HEADLESS=false' \
  'SMTP_HOST=smtp-relay.brevo.com' \
  'SMTP_PORT=587' \
  'SMTP_SECURE=false' \
  'SMTP_REQUIRE_TLS=true' \
  'SMTP_USER=fixture-brevo-user' \
  'SMTP_PASSWORD=fixture-brevo-key' \
  'SMTP_FROM_EMAIL=noreply@filfinance.fr' \
  'SMTP_FROM_NAME=FilFinance' \
  'NEWS_FINANCIAL_JUICE_EMAIL=operations@filfinance.fr' \
  'NEWS_FINANCIAL_JUICE_PASSWORD=local-fixture-password' \
  'NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN=local-fixture-token' \
  'OPENROUTER_API_KEY=local-fixture-openrouter-key' \
  > "$fixture"

bash "$CHECK_SCRIPT" "$fixture" >/dev/null
echo 'OVH go-live preflight tests passed'
