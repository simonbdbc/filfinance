#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GATE="$ROOT_DIR/scripts/check_gitlab_release_gate.sh"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

FAKE_BIN="$TMP_DIR/bin"
FIXTURE_ROOT="$TMP_DIR/api/projects/test-project"
mkdir -p \
  "$FAKE_BIN" \
  "$FIXTURE_ROOT/resource_groups" \
  "$FIXTURE_ROOT/repository/branches"

export FILFINANCE_GITLAB_TEST_API_BASE_URL="https://gitlab.example/api/v4"
export FILFINANCE_GITLAB_TEST_FIXTURE_ROOT="$TMP_DIR/api"

printf '%s\n' \
  '#!/usr/bin/env bash' \
  'set -euo pipefail' \
  'args=("$@")' \
  'output_file=""' \
  'request_url=""' \
  'for ((index = 0; index < ${#args[@]}; index += 1)); do' \
  '  case "${args[$index]}" in' \
  '    --output) output_file="${args[$((index + 1))]}"; index=$((index + 1)) ;;' \
  '    https://*) request_url="${args[$index]}" ;;' \
  '  esac' \
  'done' \
  'case "$request_url" in' \
  '  "$FILFINANCE_GITLAB_TEST_API_BASE_URL"/*)' \
  '    fixture="$FILFINANCE_GITLAB_TEST_FIXTURE_ROOT/${request_url#"$FILFINANCE_GITLAB_TEST_API_BASE_URL"/}.json"' \
  '    [ -f "$fixture" ] || exit 22' \
  '    cp "$fixture" "$output_file"' \
  '    ;;' \
  '  *) echo "Unexpected GitLab API URL: $request_url" >&2; exit 22 ;;' \
  'esac' \
  > "$FAKE_BIN/curl"
chmod +x "$FAKE_BIN/curl"
export PATH="$FAKE_BIN:$PATH"

write_fixtures() {
  printf '%s\n' '{"key":"development","process_mode":"oldest_first"}' \
    > "$FIXTURE_ROOT/resource_groups/development.json"
  printf '%s\n' '{"key":"production","process_mode":"oldest_first"}' \
    > "$FIXTURE_ROOT/resource_groups/production.json"
  printf '%s\n' '{"name":"dev","commit":{"id":"test-sha"}}' \
    > "$FIXTURE_ROOT/repository/branches/dev.json"
  printf '%s\n' '{"name":"main","commit":{"id":"test-sha"}}' \
    > "$FIXTURE_ROOT/repository/branches/main.json"
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

export CI_API_V4_URL="$FILFINANCE_GITLAB_TEST_API_BASE_URL"
export CI_PROJECT_ID=test-project
export CI_COMMIT_BRANCH=dev
export CI_COMMIT_SHA=test-sha
export CI_COMMIT_REF_PROTECTED=true
export FILFINANCE_GITLAB_READ_API_TOKEN=test-read-token

write_fixtures
expect_pass bash "$GATE" development lock-only

write_fixtures
printf '%s\n' '{"name":"dev","commit":{"id":"newer-sha"}}' \
  > "$FIXTURE_ROOT/repository/branches/dev.json"
expect_fail_contains "is not the current dev branch head" \
  bash "$GATE" development lock-only

write_fixtures
CI_COMMIT_BRANCH=main \
  expect_fail_contains "development releases must run from dev" \
  bash "$GATE" development lock-only

write_fixtures
CI_COMMIT_BRANCH=main \
  expect_pass bash "$GATE" production lock-only

write_fixtures
CI_COMMIT_BRANCH=main \
  expect_fail_contains "mode must be lock-only" \
  bash "$GATE" production require-dev-smoke

echo "GitLab release gate tests passed"
