#!/usr/bin/env bash
set -euo pipefail

# SMTP configuration belongs exclusively to the server environment. This check
# rejects public build-time variables before they can reach an Expo, Vite, Next
# or React client bundle. It deliberately never prints the matching line: a
# future violation might otherwise disclose the value that the check rejects.
SCAN_ROOT="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"

fail() {
  echo "Public environment safety check failed: $*" >&2
  exit 1
}

[ -d "$SCAN_ROOT" ] || fail "scan root does not exist"

public_prefix='(EXPO_PUBLIC|NEXT_PUBLIC|VITE|PUBLIC|REACT_APP)'

has_match() {
  local expression="$1"

  if command -v rg >/dev/null 2>&1; then
    rg -q --hidden \
      --glob '!**/node_modules/**' \
      --glob '!**/.git/**' \
      "$expression" \
      "$SCAN_ROOT"
    return
  fi

  local file
  while IFS= read -r -d '' file; do
    if grep -E -q "$expression" "$file"; then
      return 0
    fi
  done < <(
    find "$SCAN_ROOT" \
      \( -type d \( -name node_modules -o -name .git \) -prune \) -o \
      \( -type f -print0 \)
  )

  return 1
}

if has_match "^[[:space:]]*${public_prefix}_[A-Za-z0-9_]*SMTP[A-Za-z0-9_]*[[:space:]]*="; then
  fail "SMTP configuration must not use a public environment-variable prefix"
fi

if has_match "^[[:space:]]*${public_prefix}_[A-Za-z0-9_]*BETTER_AUTH_URL[[:space:]]*=[[:space:]]*https?://filfinance-auth(:[0-9]+)?(/|[[:space:]]|$)"; then
  fail "a Docker-internal Better Auth URL must not use a public environment-variable prefix"
fi

echo "Public environment safety check passed"
