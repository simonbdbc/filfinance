#!/usr/bin/env bash
set -euo pipefail

COMPOSE_FILES="${FILFINANCE_COMPOSE_FILES:-compose.yaml:compose.prod.yaml}"
ENV_FILE="${FILFINANCE_ENV_FILE:-.env.server}"
SERVICE="${FILFINANCE_SERVICE:-filfinance-service}"
CONTAINER_BASE_URL="${FILFINANCE_CONTAINER_BASE_URL:-http://127.0.0.1:8088}"
HEALTH_RETRY_ATTEMPTS="${FILFINANCE_HEALTH_RETRY_ATTEMPTS:-30}"
HEALTH_RETRY_DELAY_SECS="${FILFINANCE_HEALTH_RETRY_DELAY_SECS:-2}"

fail() {
  echo "Backend runtime smoke failed: $*" >&2
  exit 1
}

info() {
  echo "[backend-runtime-smoke] $*"
}

dump_runtime_diagnostics() {
  local container_id

  info "docker compose service status"
  compose ps "$SERVICE" || true

  container_id="$(compose ps -q "$SERVICE" 2>/dev/null || true)"
  if [ -n "$container_id" ]; then
    info "docker container state"
    docker inspect \
      --format 'name={{.Name}} status={{.State.Status}} exit_code={{.State.ExitCode}} oom_killed={{.State.OOMKilled}} error={{.State.Error}} restart_count={{.RestartCount}} finished_at={{.State.FinishedAt}}' \
      "$container_id" || true
  fi

  info "recent backend logs"
  compose logs --no-color --tail=200 "$SERVICE" || true
}

check_auth_runtime() {
  info "checking Better Auth and PostgreSQL runtime"

  if ! compose exec -T filfinance-auth node -e \
    "Promise.all([fetch('http://127.0.0.1:3000/ready'), fetch('http://127.0.0.1:3000/api/auth/jwks')]).then(async ([ready, jwks]) => { const payload = await jwks.json(); process.exit(ready.ok && jwks.ok && Array.isArray(payload.keys) && payload.keys.length > 0 ? 0 : 1); }).catch(() => process.exit(1))"; then
    compose logs --no-color --tail=200 filfinance-auth filfinance-postgres || true
    fail "Better Auth readiness or JWKS check failed"
  fi

  if ! compose exec -T filfinance-postgres sh -c \
    'pg_isready -U "$POSTGRES_USER" -d "$POSTGRES_DB" >/dev/null'; then
    fail "FilFinance PostgreSQL is not ready"
  fi
}

check_gateway_runtime() {
  if ! compose config --services | grep -qx "filfinance-gateway"; then
    info "skipping gateway checks because filfinance-gateway is not in the compose stack"
    return
  fi

  info "checking API gateway routing"

  if ! compose exec -T filfinance-gateway nginx -t; then
    compose logs --no-color --tail=200 filfinance-gateway || true
    fail "FilFinance gateway configuration is invalid"
  fi

  if ! compose exec -T filfinance-gateway sh -c '
    set -eu
    wget -q -O /dev/null http://127.0.0.1:8080/health
    wget -q -O - http://127.0.0.1:8080/api/auth/jwks | grep -q "\"keys\""
  '; then
    compose logs --no-color --tail=200 filfinance-gateway filfinance-auth filfinance-service || true
    fail "FilFinance gateway does not route backend health and Better Auth JWKS"
  fi
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    fail "missing required command: $1"
  fi
}

compose() {
  local compose_args=()
  local compose_file

  # Compose accepte plusieurs fichiers `-f`, separes ici par `:`.
  IFS=':' read -r -a compose_files <<< "$COMPOSE_FILES"
  for compose_file in "${compose_files[@]}"; do
    compose_args+=("-f" "$compose_file")
  done

  docker compose --env-file "$ENV_FILE" "${compose_args[@]}" "$@"
}

container_env() {
  local name="$1"

  compose exec -T "$SERVICE" sh -c 'printenv "$1" 2>/dev/null || true' sh "$name" | tr -d '\r'
}

service_env() {
  local service="$1"
  local name="$2"

  compose exec -T "$service" sh -c 'printenv "$1" 2>/dev/null || true' sh "$name" | tr -d '\r'
}

container_exec() {
  compose exec -T "$SERVICE" sh -c "$@"
}

is_true() {
  case "$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')" in
    true | 1 | yes | on) return 0 ;;
    *) return 1 ;;
  esac
}

assert_file_context() {
  local compose_file

  IFS=':' read -r -a compose_files <<< "$COMPOSE_FILES"
  for compose_file in "${compose_files[@]}"; do
    test -f "$compose_file" || fail "missing compose file: $compose_file"
  done

  test -f "$ENV_FILE" || fail "missing environment file: $ENV_FILE"
}

check_backend_health() {
  local attempt

  info "checking container health endpoint"

  for ((attempt = 1; attempt <= HEALTH_RETRY_ATTEMPTS; attempt += 1)); do
    if container_exec 'curl -fsS "$1/health" >/dev/null' sh "$CONTAINER_BASE_URL"; then
      return
    fi

    info "health not ready yet, retry ${attempt}/${HEALTH_RETRY_ATTEMPTS}"
    sleep "$HEALTH_RETRY_DELAY_SECS"
  done

  dump_runtime_diagnostics
  fail "backend health endpoint is not reachable inside the container"
}

resolve_tts_helper() {
  local command_args="$1"
  local helper="${command_args%%,*}"

  if [ -z "$helper" ]; then
    helper="/app/scripts/piper_local_session.py"
  fi

  printf '%s' "$helper"
}

resolve_csv_first() {
  local command_args="$1"

  printf '%s' "${command_args%%,*}"
}

check_translation_runtime() {
  local translation_provider
  local translation_command
  local translation_command_args
  local translation_helper

  translation_provider="$(container_env NEWS_TRANSLATION_PROVIDER)"

  case "$translation_provider" in
    heuristic_fr | "")
      info "skipping OPUS-MT checks because NEWS_TRANSLATION_PROVIDER=${translation_provider:-heuristic_fr}"
      return
      ;;
    opus_mt_local_session | local_command_session | opus_mt_local_command | local_command)
      ;;
    *)
      fail "unsupported translation provider in runtime smoke: $translation_provider"
      ;;
  esac

  translation_command="$(container_env NEWS_TRANSLATION_COMMAND)"
  translation_command_args="$(container_env NEWS_TRANSLATION_COMMAND_ARGS)"
  translation_helper="$(resolve_csv_first "$translation_command_args")"

  info "checking OPUS-MT translation runtime"

  if [ -z "$translation_command" ]; then
    fail "NEWS_TRANSLATION_COMMAND is empty while NEWS_TRANSLATION_PROVIDER=$translation_provider"
  fi

  if [ -z "$translation_command_args" ]; then
    fail "NEWS_TRANSLATION_COMMAND_ARGS is empty while NEWS_TRANSLATION_PROVIDER=$translation_provider"
  fi

  if ! container_exec 'test -x "$NEWS_TRANSLATION_COMMAND" || command -v "$NEWS_TRANSLATION_COMMAND" >/dev/null 2>&1'; then
    fail "NEWS_TRANSLATION_COMMAND is not executable in the container"
  fi

  if ! compose exec -T "$SERVICE" sh -c 'test -f "$1"' sh "$translation_helper"; then
    fail "OPUS-MT helper script is missing in the container: $translation_helper"
  fi

  if ! compose exec -T "$SERVICE" sh -c '
    set -eu
    args_file="$(mktemp)"
    printf "%s\n" "$NEWS_TRANSLATION_COMMAND_ARGS" | tr "," "\n" > "$args_file"

    model_dir=""
    source_spm=""
    target_spm=""
    previous=""
    while IFS= read -r arg; do
      case "$previous" in
        --ct2-model-dir) model_dir="$arg" ;;
        --sentencepiece-model) source_spm="$arg" ;;
        --target-sentencepiece-model) target_spm="$arg" ;;
      esac
      previous="$arg"
    done < "$args_file"
    rm -f "$args_file"

    test -n "$model_dir"
    test -f "$model_dir/model.bin"
    test -f "$model_dir/config.json"
    test -f "$source_spm"
    test -f "$target_spm"
  '; then
    fail "OPUS-MT model assets are missing in the container"
  fi

  if ! compose exec -T "$SERVICE" sh -c '
    set -eu
    args_file="$(mktemp)"
    printf "%s\n" "$NEWS_TRANSLATION_COMMAND_ARGS" | tr "," "\n" > "$args_file"

    require_arg_value() {
      flag="$1"
      previous=""
      value=""

      while IFS= read -r arg; do
        if [ "$previous" = "$flag" ]; then
          value="$arg"
          break
        fi
        previous="$arg"
      done < "$args_file"

      case "$value" in
        "" | --*)
          echo "missing value for $flag in NEWS_TRANSLATION_COMMAND_ARGS" >&2
          exit 1
          ;;
      esac
    }

    require_arg_value "--ct2-model-dir"
    require_arg_value "--sentencepiece-model"
    require_arg_value "--target-sentencepiece-model"
    require_arg_value "--translation-mode"

    set --
    has_session=false
    while IFS= read -r arg; do
      if [ -n "$arg" ]; then
        set -- "$@" "$arg"
        if [ "$arg" = "--session" ]; then
          has_session=true
        fi
      fi
    done < "$args_file"
    rm -f "$args_file"

    if [ "$has_session" = "false" ]; then
      case "$NEWS_TRANSLATION_PROVIDER" in
        *session) set -- "$@" --session ;;
      esac
    fi

    payload="{\"source_language\":\"en\",\"target_language\":\"fr\",\"title\":\"Federal Reserve signals rates stay higher for longer\",\"body\":null}"
    output="$(printf "%s\n" "$payload" | "$NEWS_TRANSLATION_COMMAND" "$@")"
    if printf "%s\n" "$output" | grep -q "\"error\""; then
      printf "%s\n" "$output" >&2
      exit 1
    fi
    printf "%s\n" "$output" | grep -q "\"translated_title\""
  '; then
    fail "OPUS-MT did not produce a valid translation payload"
  fi
}

check_piper_runtime() {
  local tts_provider
  local tts_command
  local tts_command_args
  local tts_helper

  tts_provider="$(container_env NEWS_TTS_PROVIDER)"
  tts_command="$(container_env NEWS_TTS_COMMAND)"
  tts_command_args="$(container_env NEWS_TTS_COMMAND_ARGS)"
  tts_helper="$(resolve_tts_helper "$tts_command_args")"

  info "checking Piper TTS runtime"

  if [ "$tts_provider" != "piper_local_session" ]; then
    fail "NEWS_TTS_PROVIDER must be piper_local_session for this smoke, got: ${tts_provider:-<empty>}"
  fi

  if [ -z "$tts_command" ]; then
    fail "NEWS_TTS_COMMAND is empty while NEWS_TTS_ENABLED=true"
  fi

  if ! container_exec 'test -x "$NEWS_TTS_COMMAND" || command -v "$NEWS_TTS_COMMAND" >/dev/null 2>&1'; then
    fail "NEWS_TTS_COMMAND is not executable in the container"
  fi

  if ! compose exec -T "$SERVICE" sh -c 'test -f "$1"' sh "$tts_helper"; then
    fail "Piper helper script is missing in the container: $tts_helper"
  fi

  if ! container_exec 'test -d "$NEWS_TTS_MODELS_DIR"'; then
    fail "NEWS_TTS_MODELS_DIR does not exist in the container"
  fi

  if ! compose exec -T "$SERVICE" sh -c '"$NEWS_TTS_COMMAND" "$1" --data-dir "$NEWS_TTS_MODELS_DIR" --validate-voice "$NEWS_TTS_VOICE" >/dev/null' sh "$tts_helper"; then
    fail "configured Piper voice is not valid"
  fi

  if ! compose exec -T "$SERVICE" sh -c '
    set -eu
    helper="$1"
    rate="${NEWS_TTS_RATE_WORDS_PER_MINUTE:-185}"
    payload="$(printf "{\"voice\":\"%s\",\"text\":\"Test audio.\",\"data_dir\":\"%s\",\"rate_words_per_minute\":%s}\n" "$NEWS_TTS_VOICE" "$NEWS_TTS_MODELS_DIR" "$rate")"
    output="$(printf "%s" "$payload" | "$NEWS_TTS_COMMAND" "$helper" --session)"
    if printf "%s\n" "$output" | grep -q "\"error\""; then
      printf "%s\n" "$output" >&2
      exit 1
    fi
    printf "%s\n" "$output" | grep -q "\"audio_base64\""
  ' sh "$tts_helper"; then
    fail "Piper did not produce a valid WAV payload"
  fi
}

check_hls_runtime() {
  local hls_access_control_enabled
  local status_code

  info "checking HLS origin runtime"

  if ! is_true "$(container_env NEWS_TTS_ENABLED)"; then
    fail "NEWS_TTS_ENABLED must be true when NEWS_HLS_ENABLED=true"
  fi

  if ! container_exec 'command -v "$NEWS_HLS_FFMPEG_COMMAND" >/dev/null 2>&1'; then
    fail "NEWS_HLS_FFMPEG_COMMAND is not available in the container"
  fi

  if ! container_exec 'mkdir -p "$NEWS_HLS_SEGMENTS_DIR" && test -w "$NEWS_HLS_SEGMENTS_DIR"'; then
    fail "NEWS_HLS_SEGMENTS_DIR is not writable in the container"
  fi

  hls_access_control_enabled="$(container_env NEWS_HLS_ACCESS_CONTROL_ENABLED)"
  if is_true "$hls_access_control_enabled"; then
    status_code="$(
      container_exec 'curl -sS -o /tmp/filfinance-hls-smoke.out -w "%{http_code}" "$1/audio/replay/today/playlist.m3u8"' sh "$CONTAINER_BASE_URL"
    )"
    if [ "$status_code" != "401" ]; then
      fail "daily replay HLS playlist should reject naked access when NEWS_HLS_ACCESS_CONTROL_ENABLED=true, got HTTP $status_code"
    fi
    return
  fi

  if ! container_exec 'curl -fsS "$1/audio/replay/today/playlist.m3u8" | grep -q "#EXTM3U"' sh "$CONTAINER_BASE_URL"; then
    fail "daily replay HLS playlist is not reachable or not valid"
  fi
}

main() {
  local tts_enabled
  local hls_enabled

  require_command docker
  assert_file_context

  check_auth_runtime
  check_backend_health
  check_gateway_runtime
  check_translation_runtime

  tts_enabled="$(container_env NEWS_TTS_ENABLED)"
  hls_enabled="$(container_env NEWS_HLS_ENABLED)"

  if is_true "$tts_enabled"; then
    case "$(container_env NEWS_TTS_PROVIDER)" in
      piper_local_session)
        check_piper_runtime
        ;;
      *)
        fail "unsupported NEWS_TTS_PROVIDER for smoke: $(container_env NEWS_TTS_PROVIDER)"
        ;;
    esac
  else
    info "skipping TTS checks because NEWS_TTS_ENABLED=false"
  fi

  if is_true "$hls_enabled"; then
    check_hls_runtime
  else
    info "skipping HLS checks because NEWS_HLS_ENABLED=false"
  fi

  info "backend runtime smoke checks passed"
}

main "$@"
