#!/usr/bin/env bash
# Simple sclang smoke test that surfaces which arch/slice runs.
set -euo pipefail

SCLANG=${1:-/Applications/SuperCollider.app/Contents/MacOS/sclang}

echo "Shell arch: $(uname -m)"
echo "SCLANG path: $SCLANG"

echo "Running default slice:"
if "$SCLANG" -v 2>&1; then
  :
else
  status=$?
  echo "Default slice failed with exit $status"
fi

if command -v arch >/dev/null 2>&1; then
  echo ""
  echo "Running arm64 slice via arch:"
  if arch -arm64 "$SCLANG" -v 2>&1; then
    :
  else
    status=$?
    echo "arm64 slice failed with exit $status"
  fi
  echo ""
  echo "Running x86_64 slice via arch:"
  if arch -x86_64 "$SCLANG" -v 2>&1; then
    :
  else
    status=$?
    echo "x86_64 slice failed with exit $status"
  fi
fi
