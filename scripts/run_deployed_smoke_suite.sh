#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

require_env() {
  local name="$1"
  [ -n "${!name:-}" ] || {
    echo "$name is required for the deployed smoke suite" >&2
    exit 2
  }
}

for name in \
  FILFINANCE_FRONTEND_URL FILFINANCE_PWA_URL FILFINANCE_API_URL FILFINANCE_EDGE_URL \
  FILFINANCE_SMOKE_LISTENER_EMAIL FILFINANCE_SMOKE_LISTENER_PASSWORD \
  FILFINANCE_SMOKE_MODERATOR_EMAIL FILFINANCE_SMOKE_MODERATOR_PASSWORD \
  FILFINANCE_SMOKE_ADMIN_EMAIL FILFINANCE_SMOKE_ADMIN_PASSWORD
do
  require_env "$name"
done

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

acquire_token() {
  local label="$1"
  local email="$2"
  local password="$3"
  local token_file="$4"

  FILFINANCE_API_BASE_URL="$FILFINANCE_API_URL" \
  FILFINANCE_PWA_URL="$FILFINANCE_PWA_URL" \
  FILFINANCE_SMOKE_EMAIL="$email" \
  FILFINANCE_SMOKE_PASSWORD="$password" \
  FILFINANCE_SMOKE_ACCOUNT_LABEL="$label" \
  FILFINANCE_SMOKE_TOKEN_FILE="$token_file" \
    node "$ROOT_DIR/scripts/acquire_better_auth_smoke_token.mjs"
}

acquire_token \
  listener \
  "$FILFINANCE_SMOKE_LISTENER_EMAIL" \
  "$FILFINANCE_SMOKE_LISTENER_PASSWORD" \
  "$TMP_DIR/listener.jwt"
acquire_token \
  moderator \
  "$FILFINANCE_SMOKE_MODERATOR_EMAIL" \
  "$FILFINANCE_SMOKE_MODERATOR_PASSWORD" \
  "$TMP_DIR/moderator.jwt"
acquire_token \
  admin \
  "$FILFINANCE_SMOKE_ADMIN_EMAIL" \
  "$FILFINANCE_SMOKE_ADMIN_PASSWORD" \
  "$TMP_DIR/admin.jwt"

export FILFINANCE_AUTH_TOKEN="$(cat "$TMP_DIR/listener.jwt")"
export FILFINANCE_LISTENER_ACCESS_TOKEN="$FILFINANCE_AUTH_TOKEN"
export FILFINANCE_MODERATOR_ACCESS_TOKEN="$(cat "$TMP_DIR/moderator.jwt")"
export FILFINANCE_ADMIN_ACCESS_TOKEN="$(cat "$TMP_DIR/admin.jwt")"
export FILFINANCE_API_BASE_URL="$FILFINANCE_API_URL"
export FILFINANCE_REQUIRE_AUTH=1

bash "$ROOT_DIR/scripts/smoke_deploy.sh" full
bash "$ROOT_DIR/filfinance-service/scripts/smoke_test_better_auth.sh"
node "$ROOT_DIR/filfinance-service/scripts/smoke_test_websocket_roles.mjs"

if [ "${FILFINANCE_NATIVE_CALENDAR_OBSERVE:-0}" = "1" ]; then
  node "$ROOT_DIR/scripts/observe_native_calendar_prod.mjs"
fi

if [ "${FILFINANCE_BUNNY_DEV_SMOKE:-0}" = "1" ]; then
  node "$ROOT_DIR/scripts/smoke_bunny_dev_surface.mjs"
fi
