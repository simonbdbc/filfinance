#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHECK_SCRIPT="$ROOT_DIR/scripts/check_no_public_smtp_secrets.sh"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

run_check() {
  # CI's Alpine image has BusyBox grep and no ripgrep. Keep this portability
  # path covered explicitly instead of relying on the local rg fast path.
  PATH='/usr/bin:/bin' bash "$CHECK_SCRIPT" "$TMP_DIR"
}

printf '%s\n' 'EXPO_PUBLIC_API_URL=https://api.filfinance.fr' > "$TMP_DIR/valid.env"
run_check >/dev/null

printf '%s\n' 'EXPO_PUBLIC_SMTP_PASSWORD=fixture-value' > "$TMP_DIR/invalid-smtp.env"
if run_check >/dev/null 2>&1; then
  echo 'public SMTP configuration was unexpectedly accepted' >&2
  exit 1
fi
rm -f "$TMP_DIR/invalid-smtp.env"

printf '%s\n' 'VITE_BETTER_AUTH_URL=http://filfinance-auth:3000' > "$TMP_DIR/invalid-internal-url.env"
if run_check >/dev/null 2>&1; then
  echo 'public internal Better Auth URL was unexpectedly accepted' >&2
  exit 1
fi

echo 'Public environment safety tests passed'
