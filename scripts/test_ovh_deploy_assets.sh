#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "test_ovh_deploy_assets: $*" >&2
  exit 1
}

contains() {
  local file="$1"
  local text="$2"
  grep -Fq -- "$text" "$file" || fail "missing expected text in $file: $text"
}

contains .env.server.example 'FILFINANCE_ENV_FILE=.env.server.prod'
contains .env.server.example 'FILFINANCE_AUTH_TRAEFIK_ENABLED=false'
contains .env.server.example 'FILFINANCE_TRAEFIK_ENABLED=false'
contains .gitlab-ci.yml '    - home-runner-docker'
contains .gitlab-ci.yml 'DEPLOY_PATH="${FILFINANCE_PROD_DEPLOY_PATH:-/srv/filfinance}"'
contains .gitlab-ci.yml 'FILFINANCE_COMPOSE_FILES=compose.yaml:compose.prod.yaml:compose.ovh.yaml'
test -f deploy/ovh/restic.env.example || fail 'inert Restic configuration template is missing'

test -f compose.ovh.yaml || fail 'OVH Compose overlay is missing'
contains compose.ovh.yaml 'networks: !override'
contains compose.ovh.yaml 'labels: !override []'
contains compose.prod.yaml '  filfinance-gateway:'
contains compose.prod.yaml '      - traefik_network'

test -f deploy/traefik/ovh/dynamic/filfinance-api.yaml.disabled \
  || fail 'dormant Traefik route is missing'
test ! -e deploy/traefik/ovh/dynamic/filfinance-api.yaml \
  || fail 'Traefik route must remain dormant in the repository'
contains deploy/traefik/ovh/dynamic/filfinance-api.yaml.disabled 'url: http://filfinance-gateway:8080'
contains deploy/traefik/ovh/compose.yaml '--providers.file.directory=/etc/traefik/dynamic'
if grep -Fq '/var/run/docker.sock' deploy/traefik/ovh/compose.yaml; then
  fail 'OVH Traefik must not mount the Docker socket'
fi

contains deploy/ovh/backup-filfinance.sh 'pg_dump -U "$POSTGRES_USER" --format=custom "$POSTGRES_DB"'
contains deploy/ovh/backup-filfinance.sh 'PRAGMA integrity_check'
contains deploy/ovh/backup-filfinance.sh 'flock -n 9'
contains deploy/ovh/backup-filfinance.sh "-f \"\$APP_DIR/compose.ovh.yaml\""
test -f scripts/check_ovh_go_live_env.sh || fail 'OVH go-live preflight is missing'
test -f scripts/bootstrap_ovh_env.sh || fail 'OVH environment bootstrap is missing'
test -f scripts/verify_ovh_runtime.sh || fail 'OVH runtime verification is missing'
contains compose.yaml 'process.kill(1, 0)'
contains scripts/verify_ovh_runtime.sh 'assert_only_gateway_reaches_traefik'
contains scripts/verify_ovh_runtime.sh 'assert_gateway_loopback_only'

echo 'OVH deployment assets test passed'
