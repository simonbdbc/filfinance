#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATA_DIR="${ROOT_DIR}/data/models/piper"
ENV_FILE="${ROOT_DIR}/.env"
VOICE_ID="${1:-}"

usage() {
  echo "Usage: ./scripts/remove_piper_voice.sh <voice_id>" >&2
  echo "Example: ./scripts/remove_piper_voice.sh fr_FR-mls-medium" >&2
  exit 1
}

fail() {
  echo "${1}" >&2
  exit 1
}

normalize_env_value() {
  local raw_value="${1:-}"
  raw_value="${raw_value%\"}"
  raw_value="${raw_value#\"}"
  printf '%s' "${raw_value}"
}

remove_env_tts_voice_line() {
  if [ ! -f "${ENV_FILE}" ]; then
    return
  fi

  local tmp_env
  tmp_env="$(mktemp "${TMPDIR:-/tmp}/filfinance-env.XXXXXX")"
  awk '!/^NEWS_TTS_VOICE=/' "${ENV_FILE}" > "${tmp_env}"
  mv "${tmp_env}" "${ENV_FILE}"
}

list_remaining_voices() {
  if [ ! -d "${DATA_DIR}" ]; then
    echo "Aucune voix restante: ${DATA_DIR} n'existe pas encore."
    return
  fi

  local voices=()
  local voice_dir
  for voice_dir in "${DATA_DIR}"/*; do
    if [ ! -d "${voice_dir}" ]; then
      continue
    fi

    if [ -f "${voice_dir}/model.onnx" ] && [ -f "${voice_dir}/model.onnx.json" ]; then
      voices+=("$(basename "${voice_dir}")")
    fi
  done

  if [ "${#voices[@]}" -eq 0 ]; then
    echo "Aucune voix Piper valide restante dans ${DATA_DIR}."
    return
  fi

  printf 'Voix restantes:\n'
  printf '  - %s\n' "${voices[@]}"
}

if [ -z "${VOICE_ID}" ]; then
  usage
fi

case "${VOICE_ID}" in
  "."|".."|*"/"* )
    fail "VOICE_ID invalide: ${VOICE_ID}"
    ;;
esac

VOICE_DIR="${DATA_DIR}/${VOICE_ID}"

if [ ! -d "${VOICE_DIR}" ]; then
  fail "Voix Piper introuvable: ${VOICE_DIR}"
fi

CURRENT_VOICE=""
if [ -f "${ENV_FILE}" ]; then
  CURRENT_VOICE="$(awk -F= '/^NEWS_TTS_VOICE=/{print substr($0, index($0, "=") + 1); exit}' "${ENV_FILE}")"
  CURRENT_VOICE="$(normalize_env_value "${CURRENT_VOICE}")"
fi

echo "Suppression de la voix Piper"
echo "  root:  ${ROOT_DIR}"
echo "  data:  ${DATA_DIR}"
echo "  voice: ${VOICE_ID}"

rm -rf "${VOICE_DIR}"

echo "Voix supprimee: ${VOICE_ID}"

if [ -n "${CURRENT_VOICE}" ] && { [ "${CURRENT_VOICE}" = "${VOICE_ID}" ] || [[ "${CURRENT_VOICE}" == "${VOICE_ID}-"* ]]; }; then
  remove_env_tts_voice_line
  echo "NEWS_TTS_VOICE supprime de ${ENV_FILE} car il pointait sur ${VOICE_ID}."
fi

list_remaining_voices
