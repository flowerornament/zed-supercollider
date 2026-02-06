#!/bin/bash
set -euo pipefail

INPUT=$(cat)
STOP_HOOK_ACTIVE=$(echo "$INPUT" | jq -r '.stop_hook_active // false')

# Prevent infinite loop
if [ "$STOP_HOOK_ACTIVE" = "true" ]; then
  exit 0
fi

# Find project root (where .claude directory lives)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$PROJECT_DIR"

# Run quality checks via just
if just check > /tmp/zed-sc-quality.log 2>&1; then
  exit 0
else
  ERRORS=$(tail -30 /tmp/zed-sc-quality.log)
  jq -n --arg errors "$ERRORS" '{"decision":"block","reason":"Quality gate failed. Fix issues:\n\n\($errors)"}'
  exit 0
fi
