#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -eq 0 ]; then
  set -- filfinance-service
fi

normalize_bool() {
  printf '%s' "${1:-false}" | tr '[:upper:]' '[:lower:]'
}

derive_piper_model_voice_id() {
  local requested_voice="$1"
  local part_one=""
  local part_two=""
  local part_three=""
  local extra=""
  local IFS="-"

  read -r part_one part_two part_three extra <<< "${requested_voice}"

  if [ -n "${part_one}" ] && [ -n "${part_two}" ] && [ -n "${part_three}" ]; then
    printf '%s-%s-%s\n' "${part_one}" "${part_two}" "${part_three}"
    return 0
  fi

  printf '%s\n' "${requested_voice}"
}

bootstrap_piper_assets_if_needed() {
  local tts_enabled
  local tts_provider
  local models_dir
  local configured_voice
  local bootstrap_voice
  local model_voice_id
  local piper_venv_dir

  tts_enabled="$(normalize_bool "${NEWS_TTS_ENABLED:-false}")"
  tts_provider="${NEWS_TTS_PROVIDER:-piper_local_session}"

  if [ "${tts_enabled}" != "true" ] || [ "${tts_provider}" != "piper_local_session" ]; then
    return 0
  fi

  models_dir="${NEWS_TTS_MODELS_DIR:-/app/data/models/piper}"
  configured_voice="${NEWS_TTS_VOICE:-fr_FR-upmc-medium-jessica}"
  bootstrap_voice="${NEWS_TTS_BOOTSTRAP_VOICE:-${configured_voice}}"
  model_voice_id="$(derive_piper_model_voice_id "${bootstrap_voice}")"
  piper_venv_dir="${PIPER_VENV_DIR:-/opt/filfinance/piper-venv}"

  if [ ! -f "${models_dir}/${model_voice_id}/model.onnx" ] \
    || [ ! -f "${models_dir}/${model_voice_id}/model.onnx.json" ]; then
    echo "Piper voice assets missing; bootstrapping ${model_voice_id} in ${models_dir}"
    PIPER_VENV_DIR="${piper_venv_dir}" \
      PIPER_DATA_DIR="${models_dir}" \
      PIPER_INSTALL_DEPS="${PIPER_INSTALL_DEPS:-false}" \
      /app/scripts/bootstrap_piper_local.sh "${model_voice_id}"
  fi

  "${piper_venv_dir}/bin/python" \
    /app/scripts/piper_local_session.py \
    --data-dir "${models_dir}" \
    --validate-voice "${configured_voice}" >/dev/null
}

bootstrap_piper_assets_if_needed

exec "$@"
