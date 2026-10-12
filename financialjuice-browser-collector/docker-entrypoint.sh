#!/usr/bin/env bash
set -euo pipefail

is_false() {
  case "${1:-}" in
    0 | false | FALSE | False | no | NO | No | off | OFF | Off) return 0 ;;
    *) return 1 ;;
  esac
}

run_with_optional_xvfb() {
  if is_false "${NEWS_FINANCIAL_JUICE_BROWSER_HEADLESS:-false}" \
    && [ -z "${DISPLAY:-}" ] \
    && [ -z "${WAYLAND_DISPLAY:-}" ] \
    && command -v xvfb-run >/dev/null 2>&1; then
    exec xvfb-run -a "$@"
  fi

  exec "$@"
}

DATA_DIR="${NEWS_FINANCIAL_JUICE_BROWSER_DATA_DIR:-/app/data}"
USER_DATA_DIR="${NEWS_FINANCIAL_JUICE_BROWSER_USER_DATA_DIR:-$DATA_DIR/financialjuice-browser-profile}"

mkdir -p "$DATA_DIR" "$USER_DATA_DIR"
rm -f "$USER_DATA_DIR"/Singleton* 2>/dev/null || true

if [ "$(id -u)" = "0" ]; then
  mkdir -p /home/pwuser
  chown -R pwuser:pwuser "$DATA_DIR" /home/pwuser 2>/dev/null || true
  export HOME=/home/pwuser
  exec runuser --preserve-environment -u pwuser -- "$0" "$@"
fi

run_with_optional_xvfb "$@"
