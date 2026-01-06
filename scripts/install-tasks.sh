#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/.zed/tasks.json"

if [[ ! -f "$SRC" ]]; then
    echo "Source tasks file not found at $SRC" >&2
    exit 1
fi

DEST_ROOT="${1:-$PWD}"
DEST_DIR="$DEST_ROOT/.zed"
DEST="$DEST_DIR/tasks.json"

mkdir -p "$DEST_DIR"

if [[ -f "$DEST" ]]; then
    echo "A tasks file already exists at $DEST; merge manually to keep your tasks." >&2
    exit 1
fi

cp "$SRC" "$DEST"
echo "Installed SuperCollider tasks to $DEST"
