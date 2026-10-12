#!/usr/bin/env bash
set -euo pipefail

RESOURCE_GROUP="${1:-}"
MODE="${2:-lock-only}"

fail() {
  echo "GitLab release gate failed: $*" >&2
  exit 1
}

for name in CI_API_V4_URL CI_PROJECT_ID CI_COMMIT_BRANCH CI_COMMIT_SHA CI_COMMIT_REF_PROTECTED FILFINANCE_GITLAB_READ_API_TOKEN; do
  [ -n "${!name:-}" ] || fail "$name is required"
done
[ "$CI_COMMIT_REF_PROTECTED" = "true" ] \
  || fail "the release branch must be protected"
case "$RESOURCE_GROUP" in
  development) expected_branch="dev" ;;
  production) expected_branch="main" ;;
  *) fail "resource group must be development or production" ;;
esac
[ "$CI_COMMIT_BRANCH" = "$expected_branch" ] \
  || fail "$RESOURCE_GROUP releases must run from $expected_branch"
case "$MODE" in
  lock-only) ;;
  *) fail "mode must be lock-only" ;;
esac

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

api_get() {
  local output_file="$1"
  local path="$2"
  shift 2

  curl --fail --silent --show-error --get \
    --header "PRIVATE-TOKEN: $FILFINANCE_GITLAB_READ_API_TOKEN" \
    "$@" \
    --output "$output_file" \
    "${CI_API_V4_URL}/projects/${CI_PROJECT_ID}${path}"
}

api_get "$TMP_DIR/resource-group.json" "/resource_groups/$RESOURCE_GROUP"
jq -e --arg key "$RESOURCE_GROUP" '
  .key == $key and .process_mode == "oldest_first"
' "$TMP_DIR/resource-group.json" >/dev/null \
  || fail "resource group $RESOURCE_GROUP must use process_mode=oldest_first"

api_get "$TMP_DIR/branch.json" "/repository/branches/$expected_branch"
jq -e \
  --arg branch "$expected_branch" \
  --arg sha "$CI_COMMIT_SHA" '
    .name == $branch and .commit.id == $sha
  ' "$TMP_DIR/branch.json" >/dev/null \
  || fail "$CI_COMMIT_SHA is not the current $expected_branch branch head"

echo "GitLab release lock is configured for $RESOURCE_GROUP"
