#!/usr/bin/env bash
set -euo pipefail

CONFIG="languages/SuperCollider/config.toml"
BANNED_KEYS=("opt_into_language_servers" "scope_opt_in_language_servers")
REQUIRED_KEYS=("name" "grammar" "path_suffixes" "line_comments" "tab_size" "hard_tabs")

if [ ! -f "$CONFIG" ]; then
  echo "Error: missing $CONFIG" >&2
  exit 1
fi

fail=0

for key in "${BANNED_KEYS[@]}"; do
  if rg -n "^[[:space:]]*${key}[[:space:]]*=" "$CONFIG" >/dev/null 2>&1; then
    echo "Error: banned key '${key}' present in $CONFIG" >&2
    fail=1
  fi
done

for key in "${REQUIRED_KEYS[@]}"; do
  if ! rg -n "^[[:space:]]*${key}[[:space:]]*=" "$CONFIG" >/dev/null 2>&1; then
    echo "Error: required key '${key}' missing in $CONFIG" >&2
    fail=1
  fi
done

if [ "$fail" -ne 0 ]; then
  echo "config validation failed" >&2
  exit 1
fi

echo "config validation passed"
