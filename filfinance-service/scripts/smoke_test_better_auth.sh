#!/usr/bin/env bash
set -euo pipefail

API_BASE_URL="${FILFINANCE_API_BASE_URL:-http://127.0.0.1:8088}"
API_BASE_URL="${API_BASE_URL%/}"
LISTENER_TOKEN="${FILFINANCE_LISTENER_ACCESS_TOKEN:-}"
MODERATOR_TOKEN="${FILFINANCE_MODERATOR_ACCESS_TOKEN:-}"
ADMIN_TOKEN="${FILFINANCE_ADMIN_ACCESS_TOKEN:-}"
VOICE="${FILFINANCE_AUTH_SMOKE_TTS_VOICE:-fr_FR-upmc-medium-jessica}"
THEME="${FILFINANCE_AUTH_SMOKE_THEME:-dark}"
LISTENER_ROLE="${FILFINANCE_LISTENER_EXPECTED_ROLE:-listener}"
MODERATOR_ROLE="${FILFINANCE_MODERATOR_EXPECTED_ROLE:-moderator}"
ADMIN_ROLE="${FILFINANCE_ADMIN_EXPECTED_ROLE:-admin}"

if [[ -z "$LISTENER_TOKEN" ]]; then
  echo "FILFINANCE_LISTENER_ACCESS_TOKEN is required" >&2
  exit 2
fi
if [[ -z "$MODERATOR_TOKEN" ]]; then
  echo "FILFINANCE_MODERATOR_ACCESS_TOKEN is required" >&2
  exit 2
fi
if [[ -z "$ADMIN_TOKEN" ]]; then
  echo "FILFINANCE_ADMIN_ACCESS_TOKEN is required" >&2
  exit 2
fi

TMP_DIR="$(mktemp -d)"
LAST_BODY="${TMP_DIR}/body.json"
trap 'rm -rf "$TMP_DIR"' EXIT

request() {
  local method="$1"
  local path="$2"
  local token="${3:-}"
  local data="${4:-}"
  local args=(-sS -o "$LAST_BODY" -w "%{http_code}" -X "$method" "${API_BASE_URL}${path}")

  if [[ -n "$token" ]]; then
    args+=(-H "Authorization: Bearer ${token}")
  fi

  if [[ -n "$data" ]]; then
    args+=(-H "Content-Type: application/json" --data "$data")
  fi

  curl "${args[@]}"
}

assert_status() {
  local label="$1"
  local expected="$2"
  local actual="$3"

  if [[ "$actual" != "$expected" ]]; then
    echo "FAIL ${label}: expected HTTP ${expected}, got HTTP ${actual}" >&2
    cat "$LAST_BODY" >&2 || true
    exit 1
  fi

  echo "OK ${label}: HTTP ${actual}"
}

assert_json_string_field() {
  local label="$1"
  local field="$2"
  local expected="$3"
  local pattern="\"${field}\"[[:space:]]*:[[:space:]]*\"${expected}\""

  if ! grep -Eq "$pattern" "$LAST_BODY"; then
    echo "FAIL ${label}: expected JSON field ${field}=${expected}" >&2
    cat "$LAST_BODY" >&2 || true
    exit 1
  fi

  echo "OK ${label}"
}

status="$(request GET /health)"
assert_status "health public" 200 "$status"

status="$(request GET /api/me)"
assert_status "profile rejects missing bearer" 401 "$status"

status="$(request GET /api/me "$LISTENER_TOKEN")"
assert_status "profile accepts listener bearer" 200 "$status"
assert_json_string_field "profile returns expected listener role" "role" "$LISTENER_ROLE"

status="$(request PATCH /api/me "$LISTENER_TOKEN" "{\"preferred_tts_voice\":\"${VOICE}\",\"theme\":\"${THEME}\"}")"
assert_status "profile stores user preferences" 200 "$status"
assert_json_string_field "profile echoes preferred TTS voice" "preferred_tts_voice" "$VOICE"
assert_json_string_field "profile echoes theme preference" "theme" "$THEME"

status="$(request PATCH /api/me "$LISTENER_TOKEN" '{"theme":"invalid-smoke-theme"}')"
assert_status "profile rejects invalid theme preference" 400 "$status"

status="$(request GET /api/runtime/tts "$LISTENER_TOKEN")"
assert_status "runtime rejects listener role" 403 "$status"

status="$(request GET /api/admin/health "$LISTENER_TOKEN")"
assert_status "admin health rejects listener role" 403 "$status"

status="$(request GET /api/me "$MODERATOR_TOKEN")"
assert_status "profile accepts moderator bearer" 200 "$status"
assert_json_string_field "profile returns expected moderator role" "role" "$MODERATOR_ROLE"

status="$(request GET /api/runtime/tts "$MODERATOR_TOKEN")"
assert_status "runtime accepts moderator role" 200 "$status"

status="$(request GET /api/admin/health "$MODERATOR_TOKEN")"
assert_status "admin health rejects moderator role" 403 "$status"

status="$(request GET /api/me "$ADMIN_TOKEN")"
assert_status "profile accepts admin bearer" 200 "$status"
assert_json_string_field "profile returns expected admin role" "role" "$ADMIN_ROLE"

status="$(request GET /api/runtime/tts "$ADMIN_TOKEN")"
assert_status "runtime accepts admin role" 200 "$status"

status="$(request GET /api/admin/health "$ADMIN_TOKEN")"
assert_status "admin health accepts admin role" 200 "$status"

echo "Better Auth smoke test completed against ${API_BASE_URL}"
