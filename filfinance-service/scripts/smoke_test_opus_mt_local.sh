#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PYTHON_BIN="$ROOT_DIR/.venv-opus-mt/bin/python"
MODEL_DIR="$ROOT_DIR/data/models/opus-mt-tc-big-en-fr-ct2-int8"
HELPER="$ROOT_DIR/scripts/opus_mt_local_command.py"

cat <<'EOF' | "$PYTHON_BIN" "$HELPER" \
  --session \
  --ct2-model-dir "$MODEL_DIR" \
  --sentencepiece-model "$MODEL_DIR/source.tokenizer.model" \
  --target-sentencepiece-model "$MODEL_DIR/target.tokenizer.model" \
  --device cpu \
  --compute-type int8 \
  --beam-size 4 \
  --translation-mode opus_mt_tc_big_local_session
{"source_language":"en","target_language":"fr","title":"Federal Reserve signals rates stay higher for longer","body":null}
{"source_language":"en","target_language":"fr","title":"Powell says EUR/USD holds above 1.08 after CPI","body":"NVDA rises +3% after earnings"}
EOF
