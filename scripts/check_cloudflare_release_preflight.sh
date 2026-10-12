#!/usr/bin/env bash
# Préflight Cloudflare Worker exclusivement production. Les sites statiques
# production sont publies sur Bunny et ne doivent pas exiger Pages.
set -euo pipefail

[ "${1:-}" = "prod" ] || { echo "Cloudflare release preflight supports prod only" >&2; exit 2; }
fail() { echo "Cloudflare production preflight: $*" >&2; exit 1; }
for name in \
  CLOUDFLARE_ACCOUNT_ID CLOUDFLARE_ZONE_ID CLOUDFLARE_API_TOKEN_WORKERS_PROD; do
  [ -n "${!name:-}" ] || fail "Missing $name"
done
echo "Cloudflare Worker production preflight passed"
