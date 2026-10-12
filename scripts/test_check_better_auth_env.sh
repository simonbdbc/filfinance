#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PREFLIGHT="$ROOT_DIR/scripts/check_better_auth_env.sh"
DEV_PWA_URL="https://app.dev.filfinance.fr"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

write_env() {
  local target="$1"
  local app_web_url="$2"
  local trusted_origins="$3"
  local require_email_verification="${4:-true}"
  local api_host

  if [ "$target" = "prod" ]; then
    api_host="api.filfinance.fr"
  else
    api_host="api.dev.filfinance.fr"
  fi

  printf '%s\n' \
    "FILFINANCE_API_HOST=$api_host" \
    "POSTGRES_DB=filfinance" \
    "POSTGRES_USER=owner" \
    "POSTGRES_PASSWORD=test-owner-password" \
    "FILFINANCE_AUTH_DB_USER=auth" \
    "FILFINANCE_AUTH_DB_PASSWORD=test-auth-password" \
    "FILFINANCE_APP_DB_USER=app" \
    "FILFINANCE_APP_DB_PASSWORD=test-app-password" \
    "AUTH_DATABASE_URL=postgresql://auth:test-auth-password@postgres:5432/filfinance" \
    "DATABASE_URL=postgresql://app:test-app-password@postgres:5432/filfinance" \
    "BETTER_AUTH_URL=https://$api_host" \
    "BETTER_AUTH_SECRET=test-secret-with-at-least-thirty-two-characters" \
    "BETTER_AUTH_JWT_AUDIENCE=https://$api_host" \
    "BETTER_AUTH_JWT_EXPIRATION=15m" \
    "BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION=$require_email_verification" \
    "BETTER_AUTH_TRUSTED_ORIGINS=$trusted_origins" \
    "FILFINANCE_LANDING_USES_BETTER_AUTH=false" \
    "APP_WEB_URL=$app_web_url" \
    "EXPO_SCHEME=filfinance" \
    "SMTP_HOST=smtp-relay.brevo.com" \
    "SMTP_PORT=587" \
    "SMTP_SECURE=false" \
    "SMTP_REQUIRE_TLS=true" \
    "SMTP_FROM_EMAIL=noreply@filfinance.fr" \
    "SMTP_FROM_NAME=FilFinance" \
    "SMTP_USER=fixture-brevo-user" \
    "SMTP_PASSWORD=fixture-brevo-key" \
    "NEWS_AUTH_ENABLED=true" \
    "NEWS_AUTH_PROVIDER=better_auth" \
    "NEWS_AUTH_ISSUER=https://$api_host" \
    "NEWS_AUTH_JWKS_URL=http://filfinance-auth:3000/api/auth/jwks" \
    "NEWS_AUTH_AUDIENCE=https://$api_host" \
    "NEWS_AUTH_MODERATOR_ROLES=moderator,admin" \
    > "$TMP_DIR/$target.env"
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

prod_origins="filfinance://,https://app.filfinance.fr"
dev_origins="filfinance://,$DEV_PWA_URL"

write_env dev "$DEV_PWA_URL" "$dev_origins"
expect_pass bash "$PREFLIGHT" "$TMP_DIR/dev.env" dev "$DEV_PWA_URL"

write_env prod "https://app.filfinance.fr" "$prod_origins"
# La preflight PROD est autonome : l'environnement DEV est en pause et son
# hostname ne doit pas devenir un prerequis de la livraison PROD.
FILFINANCE_DEV_PWA_URL= \
  expect_pass bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod

write_env dev "$DEV_PWA_URL" "$dev_origins"
sed -i.bak 's/^EXPO_SCHEME=.*/EXPO_SCHEME=another-app/' "$TMP_DIR/dev.env"
expect_fail_contains "EXPO_SCHEME must equal filfinance" bash "$PREFLIGHT" "$TMP_DIR/dev.env" dev "$DEV_PWA_URL"

# Neutralise la variable CI globale afin de tester réellement l'absence du troisième argument.
expect_fail_contains "FILFINANCE_DEV_PWA_URL must be an HTTPS origin" env FILFINANCE_DEV_PWA_URL= bash "$PREFLIGHT" "$TMP_DIR/dev.env" dev
expect_fail_contains "origin without path" bash "$PREFLIGHT" "$TMP_DIR/dev.env" dev "$DEV_PWA_URL/path"
expect_fail_contains "FILFINANCE_DEV_PWA_URL must equal https://app.dev.filfinance.fr" \
  bash "$PREFLIGHT" "$TMP_DIR/dev.env" dev "https://pwa-dev.filfinance.fr"

write_env dev "https://dev.filfinance.fr" "$dev_origins"
expect_fail_contains "APP_WEB_URL must equal" bash "$PREFLIGHT" "$TMP_DIR/dev.env" dev "$DEV_PWA_URL"

write_env prod "https://app.filfinance.fr" "https://app.filfinance.fr"
expect_fail_contains "filfinance:// must be listed" bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod "$DEV_PWA_URL"

write_env prod "https://app.filfinance.fr" "$prod_origins,$DEV_PWA_URL"
expect_fail_contains "is not an allowed deployed Better Auth origin" bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod "$DEV_PWA_URL"

write_env dev "$DEV_PWA_URL" "$dev_origins,https://app.filfinance.fr"
expect_fail_contains "belongs to the other environment" bash "$PREFLIGHT" "$TMP_DIR/dev.env" dev "$DEV_PWA_URL"

write_env dev "$DEV_PWA_URL" "$dev_origins" false
expect_fail_contains "BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION must equal true" bash "$PREFLIGHT" "$TMP_DIR/dev.env" dev "$DEV_PWA_URL"

write_env dev "$DEV_PWA_URL" "$dev_origins"
sed -i.bak 's/^FILFINANCE_API_HOST=.*/FILFINANCE_API_HOST=api.filfinance.fr/' "$TMP_DIR/dev.env"
expect_fail_contains "FILFINANCE_API_HOST must equal api.dev.filfinance.fr" bash "$PREFLIGHT" "$TMP_DIR/dev.env" dev "$DEV_PWA_URL"

write_env prod "https://app.filfinance.fr" "$prod_origins"
sed -i.bak 's/^SMTP_PORT=.*/SMTP_PORT=-1/' "$TMP_DIR/prod.env"
expect_fail_contains "SMTP_PORT must be an integer" bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod "$DEV_PWA_URL"

write_env prod "https://app.filfinance.fr" "$prod_origins"
sed -i.bak 's/^SMTP_SECURE=.*/SMTP_SECURE=treu/' "$TMP_DIR/prod.env"
expect_fail_contains "SMTP_SECURE must equal true or false" bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod "$DEV_PWA_URL"

write_env prod "https://app.filfinance.fr" "$prod_origins"
sed -i.bak 's/^SMTP_REQUIRE_TLS=.*/SMTP_REQUIRE_TLS=false/' "$TMP_DIR/prod.env"
expect_fail_contains "SMTP_REQUIRE_TLS must equal true" bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod "$DEV_PWA_URL"

write_env prod "https://app.filfinance.fr" "$prod_origins,https://filfinance.fr"
expect_fail_contains "https://filfinance.fr must not be listed" bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod "$DEV_PWA_URL"

write_env prod "https://app.filfinance.fr" "$prod_origins"
sed -i.bak 's/^BETTER_AUTH_JWT_EXPIRATION=.*/BETTER_AUTH_JWT_EXPIRATION=24h/' "$TMP_DIR/prod.env"
expect_fail_contains "BETTER_AUTH_JWT_EXPIRATION must equal 15m" bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod "$DEV_PWA_URL"

write_env prod "https://app.filfinance.fr" "$prod_origins"
sed -i.bak 's/^NEWS_AUTH_MODERATOR_ROLES=.*/NEWS_AUTH_MODERATOR_ROLES=listener,moderator,admin/' "$TMP_DIR/prod.env"
expect_fail_contains "NEWS_AUTH_MODERATOR_ROLES must contain exactly moderator,admin" bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod "$DEV_PWA_URL"

write_env prod "https://app.filfinance.fr" "$prod_origins"
sed -i.bak 's/^NEWS_AUTH_MODERATOR_ROLES=.*/NEWS_AUTH_MODERATOR_ROLES=support,moderator,admin/' "$TMP_DIR/prod.env"
expect_fail_contains "NEWS_AUTH_MODERATOR_ROLES must contain exactly moderator,admin" bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod "$DEV_PWA_URL"

write_env prod "https://app.filfinance.fr" "$prod_origins,https://*.filfinance.fr"
expect_fail_contains "is not an allowed deployed Better Auth origin" bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod "$DEV_PWA_URL"

write_env prod "https://app.filfinance.fr" "$prod_origins"
sed -i.bak 's/^SMTP_USER=.*/SMTP_USER=filfinance/' "$TMP_DIR/prod.env"
sed -i.bak 's/^SMTP_PASSWORD=.*/SMTP_PASSWORD=replace-with-smtp-password/' "$TMP_DIR/prod.env"
expect_fail_contains "SMTP_PASSWORD still contains a placeholder" bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod "$DEV_PWA_URL"

write_env prod "https://app.filfinance.fr" "$prod_origins"
sed -i.bak 's/^SMTP_HOST=.*/SMTP_HOST=mail.sbcenter.fr/' "$TMP_DIR/prod.env"
expect_fail_contains "SMTP_HOST must equal smtp-relay.brevo.com" bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod "$DEV_PWA_URL"

write_env prod "https://app.filfinance.fr" "$prod_origins"
sed -i.bak 's/^SMTP_FROM_EMAIL=.*/SMTP_FROM_EMAIL=no-reply@filfinance.fr/' "$TMP_DIR/prod.env"
expect_fail_contains "SMTP_FROM_EMAIL must equal noreply@filfinance.fr" bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod "$DEV_PWA_URL"

write_env prod "https://app.filfinance.fr" "$prod_origins"
sed -i.bak 's/^FILFINANCE_APP_DB_USER=.*/FILFINANCE_APP_DB_USER=auth/' "$TMP_DIR/prod.env"
expect_fail_contains "must be distinct" bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod "$DEV_PWA_URL"

write_env prod "https://app.filfinance.fr" "$prod_origins"
sed -i.bak 's#^AUTH_DATABASE_URL=.*#AUTH_DATABASE_URL=postgresql://wrong:test-auth-password@postgres:5432/filfinance#' "$TMP_DIR/prod.env"
expect_fail_contains "AUTH_DATABASE_URL username must match" bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod "$DEV_PWA_URL"

write_env prod "https://app.filfinance.fr" "$prod_origins"
printf '%s\n' 'ADMIN_EMAIL_TO_PROMOTE=admin@example.com' >> "$TMP_DIR/prod.env"
expect_fail_contains "ADMIN_EMAIL_TO_PROMOTE still contains a placeholder" bash "$PREFLIGHT" "$TMP_DIR/prod.env" prod "$DEV_PWA_URL"

echo "Better Auth environment preflight tests passed"
