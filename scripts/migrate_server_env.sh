#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-${FILFINANCE_ENV_FILE:-.env.server}}"
MIGRATED=false
HAS_TRANSLATION_PROFILE=false
HAS_BROWSER_HEADLESS=false
HAS_TRAEFIK_CERT_RESOLVER=false
LEGACY_BEAM_SIZE=""
SANITIZED_TRANSLATION_ARGS=""

fail() {
  echo "Server env migration failed: $*" >&2
  exit 1
}

translation_profile_from_beam_size() {
  case "$1" in
    1) printf '%s' "fast" ;;
    2) printf '%s' "balanced" ;;
    4) printf '%s' "quality" ;;
    *) printf '%s' "quality" ;;
  esac
}

sanitize_translation_args() {
  local raw_value="$1"
  local quote=""
  local value="$raw_value"
  local token
  local skip_next=false
  local joined=""
  local output=()
  local tokens=()

  case "$value" in
    \"*\")
      quote='"'
      value="${value#\"}"
      value="${value%\"}"
      ;;
    \'*\')
      quote="'"
      value="${value#\'}"
      value="${value%\'}"
      ;;
  esac

  if [ -z "$value" ]; then
    SANITIZED_TRANSLATION_ARGS="${quote}${value}${quote}"
    return
  fi

  IFS=',' read -r -a tokens <<< "$value"
  for token in "${tokens[@]}"; do
    if [ "$skip_next" = true ]; then
      LEGACY_BEAM_SIZE="$token"
      skip_next=false
      continue
    fi

    case "$token" in
      --beam-size)
        MIGRATED=true
        skip_next=true
        continue
        ;;
      --beam-size=*)
        MIGRATED=true
        LEGACY_BEAM_SIZE="${token#--beam-size=}"
        continue
        ;;
    esac

    output+=("$token")
  done

  for token in "${output[@]}"; do
    if [ -n "$joined" ]; then
      joined="${joined},${token}"
    else
      joined="$token"
    fi
  done

  SANITIZED_TRANSLATION_ARGS="${quote}${joined}${quote}"
}

test -f "$ENV_FILE" || fail "missing environment file: $ENV_FILE"

tmp_file="$(mktemp "${ENV_FILE}.tmp.XXXXXX")"
cleanup() {
  rm -f "$tmp_file"
}
trap cleanup EXIT

while IFS= read -r line || [ -n "$line" ]; do
  case "$line" in
    OPENROUTER_MODEL=nvidia/nemotron-3-super-120b-a12b:free | 'OPENROUTER_MODEL="nvidia/nemotron-3-super-120b-a12b:free"' | "OPENROUTER_MODEL='nvidia/nemotron-3-super-120b-a12b:free'" | OPENROUTER_MODEL=minimax/minimax-m3:free | 'OPENROUTER_MODEL="minimax/minimax-m3:free"' | "OPENROUTER_MODEL='minimax/minimax-m3:free'")
      case "$ENV_FILE" in
        *.dev) printf '%s\n' "$line" >> "$tmp_file" ;;
        *)
          MIGRATED=true
          printf 'OPENROUTER_MODEL=dots-studio/dots-3-note-preview:free\n' >> "$tmp_file"
          ;;
      esac
      ;;
    N8N_*=* | FILFINANCE_N8N_*=* | FILFINANCE_CALENDAR_NATIVE_SHADOW_*=* | FILFINANCE_CALENDAR_SHADOW_*=*)
      MIGRATED=true
      ;;
    NEWS_PROFILE_STORE=* | LOGTO_*=* | EXPO_PUBLIC_LOGTO_*=* | SUPABASE_*=* | EXPO_PUBLIC_SUPABASE_*=*)
      MIGRATED=true
      ;;
    NEWS_AUTH_PROVIDER=*)
      if [ "$line" != "NEWS_AUTH_PROVIDER=better_auth" ]; then
        MIGRATED=true
      fi
      printf 'NEWS_AUTH_PROVIDER=better_auth\n' >> "$tmp_file"
      ;;
    NEWS_TRANSLATION_PROFILE=*)
      HAS_TRANSLATION_PROFILE=true
      printf '%s\n' "$line" >> "$tmp_file"
      ;;
    NEWS_TRANSLATION_COMMAND_ARGS=*)
      sanitize_translation_args "${line#NEWS_TRANSLATION_COMMAND_ARGS=}"
      printf 'NEWS_TRANSLATION_COMMAND_ARGS=%s\n' "$SANITIZED_TRANSLATION_ARGS" >> "$tmp_file"
      ;;
    NEWS_FINANCIAL_JUICE_BROWSER_HEADLESS=*)
      HAS_BROWSER_HEADLESS=true
      if [ "${line#NEWS_FINANCIAL_JUICE_BROWSER_HEADLESS=}" != "false" ]; then
        MIGRATED=true
      fi
      printf 'NEWS_FINANCIAL_JUICE_BROWSER_HEADLESS=false\n' >> "$tmp_file"
      ;;
    FILFINANCE_TRAEFIK_CERT_RESOLVER=*)
      HAS_TRAEFIK_CERT_RESOLVER=true
      printf '%s\n' "$line" >> "$tmp_file"
      ;;
    *)
      printf '%s\n' "$line" >> "$tmp_file"
      ;;
  esac
done < "$ENV_FILE"

if [ "$HAS_TRANSLATION_PROFILE" = false ]; then
  MIGRATED=true
  printf 'NEWS_TRANSLATION_PROFILE=%s\n' "$(translation_profile_from_beam_size "$LEGACY_BEAM_SIZE")" >> "$tmp_file"
fi

if [ "$HAS_BROWSER_HEADLESS" = false ]; then
  MIGRATED=true
  printf 'NEWS_FINANCIAL_JUICE_BROWSER_HEADLESS=false\n' >> "$tmp_file"
fi

if [ "$HAS_TRAEFIK_CERT_RESOLVER" = false ]; then
  MIGRATED=true
  case "$ENV_FILE" in
    *.dev) printf 'FILFINANCE_TRAEFIK_CERT_RESOLVER=bunny_dns\n' >> "$tmp_file" ;;
    *) printf 'FILFINANCE_TRAEFIK_CERT_RESOLVER=lets_encrypt\n' >> "$tmp_file" ;;
  esac
fi

if [ "$MIGRATED" = true ]; then
  backup_file="${ENV_FILE}.bak-$(date +%Y%m%d%H%M%S)"
  cp -p "$ENV_FILE" "$backup_file"
  cp "$tmp_file" "$ENV_FILE"
  rm -f "$tmp_file"
  trap - EXIT
  echo "Migrated server env defaults; backup created at $backup_file"
else
  rm -f "$tmp_file"
  trap - EXIT
  echo "Server env defaults are already up to date"
fi
