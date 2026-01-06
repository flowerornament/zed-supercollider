#!/usr/bin/env bash
set -euo pipefail

ROOT="${ZED_WORKTREE_ROOT:-$(pwd)}"
DEV_LAUNCHER="$ROOT/server/launcher/target/release/sc_launcher"
LAUNCHER="${SC_LAUNCHER_PATH:-}"

if [[ -z "$LAUNCHER" && -x "$DEV_LAUNCHER" ]]; then
    LAUNCHER="$DEV_LAUNCHER"
fi

if [[ -z "$LAUNCHER" ]]; then
    LAUNCHER="$(command -v sc_launcher 2>/dev/null || true)"
fi

if [[ -z "$LAUNCHER" ]]; then
    echo "Launcher not found (cwd=$PWD root=$ROOT): build server/launcher (cargo build --release) or set SC_LAUNCHER_PATH"
    exit 1
fi

echo "Using launcher: $LAUNCHER"
exec "$LAUNCHER" --mode probe
