#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VENV_DIR="${PIPER_VENV_DIR:-${ROOT_DIR}/.venv-piper}"
DATA_DIR="${PIPER_DATA_DIR:-${ROOT_DIR}/data/models/piper}"
VOICE_ID="${1:-fr_FR-upmc-medium}"
PYTHON_BIN="${PYTHON_BIN:-python3}"
PIPER_INSTALL_DEPS="${PIPER_INSTALL_DEPS:-true}"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/filfinance-piper-bootstrap.XXXXXX")"

cleanup() {
  rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

echo "Bootstrap Piper local"
echo "  root:  ${ROOT_DIR}"
echo "  venv:  ${VENV_DIR}"
echo "  data:  ${DATA_DIR}"
echo "  voice: ${VOICE_ID}"

if [ ! -d "${VENV_DIR}" ] && [ "${PIPER_INSTALL_DEPS}" = "false" ]; then
  echo "PIPER_INSTALL_DEPS=false mais le venv Piper est absent: ${VENV_DIR}" >&2
  exit 1
fi

if [ ! -d "${VENV_DIR}" ]; then
  "${PYTHON_BIN}" -m venv "${VENV_DIR}"
fi

if [ "${PIPER_INSTALL_DEPS}" != "false" ]; then
  "${VENV_DIR}/bin/python" -m pip install --upgrade pip
  "${VENV_DIR}/bin/python" -m pip install -r "${ROOT_DIR}/scripts/requirements-piper.txt"
fi

mkdir -p "${DATA_DIR}/${VOICE_ID}"

"${VENV_DIR}/bin/python" -m piper.download_voices --data-dir "${TMP_DIR}" "${VOICE_ID}"

MODEL_SRC="$(find "${TMP_DIR}" -name "${VOICE_ID}.onnx" -print -quit)"
CONFIG_SRC="$(find "${TMP_DIR}" -name "${VOICE_ID}.onnx.json" -print -quit)"

if [ -z "${MODEL_SRC}" ] || [ -z "${CONFIG_SRC}" ]; then
  echo "Impossible de trouver les assets Piper telecharges pour ${VOICE_ID}" >&2
  exit 1
fi

cp "${MODEL_SRC}" "${DATA_DIR}/${VOICE_ID}/model.onnx"
cp "${CONFIG_SRC}" "${DATA_DIR}/${VOICE_ID}/model.onnx.json"

"${VENV_DIR}/bin/python" \
  "${ROOT_DIR}/scripts/piper_local_session.py" \
  --data-dir "${DATA_DIR}" \
  --validate-voice "${VOICE_ID}"

"${VENV_DIR}/bin/python" \
  "${ROOT_DIR}/scripts/piper_local_session.py" \
  --data-dir "${DATA_DIR}" \
  --list-voices

echo "Bootstrap Piper termine pour ${VOICE_ID}"
