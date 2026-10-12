#!/usr/bin/env bash
set -euo pipefail

SMOKE_SCOPE="${FILFINANCE_SMOKE_SCOPE:-${1:-full}}"
FRONTEND_URL="${FILFINANCE_FRONTEND_URL:-https://filfinance.fr}"
PWA_URL="${FILFINANCE_PWA_URL:-https://app.filfinance.fr}"
API_URL="${FILFINANCE_API_URL:-https://api.filfinance.fr}"
EDGE_URL="${FILFINANCE_EDGE_URL:-https://live.filfinance.fr}"
CHECK_EDGE="${FILFINANCE_CHECK_EDGE:-0}"
AUTH_TOKEN="${FILFINANCE_AUTH_TOKEN:-}"
REQUIRE_AUTH="${FILFINANCE_REQUIRE_AUTH:-0}"

FRONTEND_URL="${FRONTEND_URL%/}"
PWA_URL="${PWA_URL%/}"
API_URL="${API_URL%/}"
EDGE_URL="${EDGE_URL%/}"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

require_valid_scope() {
  case "$SMOKE_SCOPE" in
    full|frontend)
      ;;
    *)
      echo "Invalid FILFINANCE_SMOKE_SCOPE: $SMOKE_SCOPE" >&2
      echo "Expected: full or frontend" >&2
      exit 1
      ;;
  esac
}

fetch_html() {
  local url="$1"
  local output="$2"

  curl -fsSL "$url" -o "$output"
}

assert_contains() {
  local file="$1"
  local pattern="$2"
  local label="$3"

  if ! grep -Fq -- "$pattern" "$file"; then
    echo "Smoke failed: $label" >&2
    echo "Expected pattern '$pattern' in $file" >&2
    exit 1
  fi
}

assert_not_contains() {
  local file="$1"
  local pattern="$2"
  local label="$3"

  if grep -Fq -- "$pattern" "$file"; then
    echo "Smoke failed: $label" >&2
    echo "Unexpected pattern '$pattern' in $file" >&2
    exit 1
  fi
}

check_frontend() {
  local root_html="$TMP_DIR/frontend-root.html"

  echo "Checking landing root: $FRONTEND_URL"
  fetch_html "$FRONTEND_URL" "$root_html"
  assert_contains "$root_html" "Fil Finance" "landing root should identify Fil Finance"
  assert_contains "$root_html" "rel=\"canonical\" href=\"${FRONTEND_URL}/\"" "landing canonical should match the requested environment"
  assert_contains "$root_html" "/_astro/" "landing root should load the Astro artifact"
  assert_not_contains "$root_html" "_expo/static" "landing root must not serve the legacy Expo artifact"
}

check_pwa() {
  local root_html="$TMP_DIR/pwa-root.html"
  local deep_html="$TMP_DIR/pwa-deep.html"
  local verification_html="$TMP_DIR/pwa-email-verification.html"

  echo "Checking PWA root: $PWA_URL"
  fetch_html "$PWA_URL" "$root_html"
  assert_contains "$root_html" "_expo/static" "PWA root should load Expo assets"

  echo "Checking PWA SPA fallback: $PWA_URL/profile"
  fetch_html "$PWA_URL/profile" "$deep_html"
  assert_contains "$deep_html" "_expo/static" "PWA deep route should return the app shell"

  echo "Checking public email verification route: $PWA_URL/email-verification"
  fetch_html "$PWA_URL/email-verification?status=success" "$verification_html"
  assert_contains "$verification_html" "_expo/static" "email verification should return the PWA shell, never raw JSON"
}

check_backend_health() {
  echo "Checking backend health: $API_URL/health"
  curl -fsSL "$API_URL/health" >/dev/null

  echo "Checking Better Auth JWKS: $API_URL/api/auth/jwks"
  curl -fsSL "$API_URL/api/auth/jwks" | grep -q '"keys"'
}

check_edge_health() {
  if [ "$CHECK_EDGE" != "1" ]; then
    echo "Skipping edge smoke check: FILFINANCE_CHECK_EDGE is not 1"
    return
  fi

  echo "Checking edge health: $EDGE_URL/health"
  curl -fsSL "$EDGE_URL/health" | grep -q '"status":"ok"'
}

check_authenticated_endpoints() {
  if [ -z "$AUTH_TOKEN" ]; then
    if [ "$REQUIRE_AUTH" = "1" ]; then
      echo "Authenticated smoke checks are required but FILFINANCE_AUTH_TOKEN is not set" >&2
      exit 1
    fi
    echo "Skipping authenticated smoke checks: FILFINANCE_AUTH_TOKEN is not set"
    return
  fi

  echo "Checking authenticated profile endpoint: $API_URL/api/me"
  curl -fsSL \
    -H "Authorization: Bearer $AUTH_TOKEN" \
    "$API_URL/api/me" \
    >/dev/null

  echo "Checking authenticated translation history endpoint"
  curl -fsSL \
    -H "Authorization: Bearer $AUTH_TOKEN" \
    "$API_URL/api/translations?history_scope=real_feed&source=financial_juice&limit=1" \
    >/dev/null
}

main() {
  require_valid_scope
  require_command curl
  require_command grep

  check_frontend
  check_pwa
  if [ "$SMOKE_SCOPE" = "frontend" ]; then
    echo "Fil Finance frontend smoke checks passed"
    return
  fi

  check_backend_health
  check_edge_health
  check_authenticated_endpoints

  echo "Fil Finance deploy smoke checks passed"
}

main "$@"
