#!/usr/bin/env bash
set -euo pipefail

# Run on the OVH VPS after `docker compose up` and before DNS is changed. It
# checks the parts Docker cannot prove from a static Compose rendering: health,
# restarts, OOM state, published ports and attachment to traefik_network.
ENV_FILE="${FILFINANCE_ENV_FILE:-.env.server.prod}"
COMPOSE_FILES="${FILFINANCE_COMPOSE_FILES:-compose.yaml:compose.prod.yaml:compose.ovh.yaml}"
EXPECTED_BACKEND_PORT="${FILFINANCE_BACKEND_PORT:-8088}"

fail() {
  echo "OVH runtime verification failed: $*" >&2
  exit 1
}

info() {
  echo "[ovh-runtime-verify] $*"
}

compose() {
  local args=()
  local compose_file
  local files=()

  IFS=':' read -r -a files <<< "$COMPOSE_FILES"
  for compose_file in "${files[@]}"; do
    test -f "$compose_file" || fail "missing Compose file: $compose_file"
    args+=("-f" "$compose_file")
  done

  docker compose --env-file "$ENV_FILE" "${args[@]}" "$@"
}

container_id() {
  local service="$1"
  local id

  id="$(compose ps -q "$service")"
  [ -n "$id" ] || fail "container is missing for service: $service"
  printf '%s' "$id"
}

assert_healthy() {
  local service="$1"
  local id state health restarts oom

  id="$(container_id "$service")"
  state="$(docker inspect --format '{{.State.Status}}' "$id")"
  health="$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}missing{{end}}' "$id")"
  restarts="$(docker inspect --format '{{.RestartCount}}' "$id")"
  oom="$(docker inspect --format '{{.State.OOMKilled}}' "$id")"

  [ "$state" = running ] || fail "$service is not running (state=$state)"
  [ "$health" = healthy ] || fail "$service is not healthy (health=$health)"
  [ "$restarts" = 0 ] || fail "$service restarted $restarts time(s)"
  [ "$oom" = false ] || fail "$service was OOM-killed"
}

published_bindings() {
  local service="$1"
  local id

  id="$(container_id "$service")"
  docker inspect --format '{{range $port, $bindings := .NetworkSettings.Ports}}{{if $bindings}}{{range $bindings}}{{printf "%s %s %s\n" $port .HostIp .HostPort}}{{end}}{{end}}{{end}}' "$id"
}

assert_no_published_ports() {
  local service="$1"
  local bindings

  bindings="$(published_bindings "$service")"
  [ -z "$bindings" ] || fail "$service publishes port(s): $bindings"
}

assert_gateway_loopback_only() {
  local bindings expected

  bindings="$(published_bindings filfinance-gateway)"
  expected="8080/tcp 127.0.0.1 $EXPECTED_BACKEND_PORT"
  [ "$bindings" = "$expected" ] || fail "gateway must bind only $expected; got: ${bindings:-<none>}"
}

network_names() {
  local service="$1"
  local id

  id="$(container_id "$service")"
  docker inspect --format '{{range $name, $_ := .NetworkSettings.Networks}}{{println $name}}{{end}}' "$id"
}

assert_only_gateway_reaches_traefik() {
  local service networks

  for service in filfinance-postgres filfinance-redis filfinance-auth filfinance-service financialjuice-browser-collector; do
    networks="$(network_names "$service")"
    if printf '%s\n' "$networks" | grep -Fxq traefik_network; then
      fail "$service must not join traefik_network"
    fi
  done

  networks="$(network_names filfinance-gateway)"
  printf '%s\n' "$networks" | grep -Fxq traefik_network \
    || fail 'filfinance-gateway is not attached to traefik_network'
}

assert_financialjuice_control() {
  compose exec -T financialjuice-browser-collector node -e '
    const base = process.env.NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL;
    const token = process.env.NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN;
    if (!base || !token) process.exit(1);
    fetch(`${base}/api/ingest/financial-juice/browser/control`, {
      headers: { authorization: `Bearer ${token}` },
    })
      .then(async (response) => {
        const payload = await response.json();
        process.exit(response.ok && payload.configured_enabled === true ? 0 : 1);
      })
      .catch(() => process.exit(1));
  ' || fail 'FinancialJuice collector cannot authenticate to its backend control endpoint'
}

main() {
  local service

  command -v docker >/dev/null 2>&1 || fail 'docker is required'
  test -r "$ENV_FILE" || fail "environment file is not readable: $ENV_FILE"
  compose config >/dev/null

  bash scripts/check_deploy_env.sh "$ENV_FILE" prod
  bash scripts/check_better_auth_env.sh "$ENV_FILE" prod

  for service in filfinance-postgres filfinance-redis filfinance-auth filfinance-service financialjuice-browser-collector filfinance-gateway; do
    assert_healthy "$service"
  done

  for service in filfinance-postgres filfinance-redis filfinance-auth filfinance-service financialjuice-browser-collector; do
    assert_no_published_ports "$service"
  done
  assert_gateway_loopback_only
  assert_only_gateway_reaches_traefik
  assert_financialjuice_control

  info 'running application smoke suite'
  FILFINANCE_ENV_FILE="$ENV_FILE" FILFINANCE_COMPOSE_FILES="$COMPOSE_FILES" \
    bash scripts/smoke_backend_runtime.sh
  info 'OVH local runtime verification passed'
}

main "$@"
