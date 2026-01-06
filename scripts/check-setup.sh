#!/usr/bin/env bash
# If this script fails in a sandboxed environment, rerun with elevated permissions
# so it can access the system sclang (e.g. /Applications/SuperCollider.app/...).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Prefer an explicit worktree root if provided; otherwise default to the repo root (one level up from scripts/).
DEFAULT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ROOT="${ZED_WORKTREE_ROOT:-$DEFAULT_ROOT}"
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
if command -v file >/dev/null 2>&1; then
    echo "Launcher binary info: $(file -b "$LAUNCHER" 2>/dev/null || true)"
fi

# Surface the probed sclang/launcher errors instead of silently exec-ing.
set +e
"$LAUNCHER" --mode probe
status=$?
set -e

if [[ $status -ne 0 ]]; then
    echo ""
    echo "Check failed (exit $status). Common causes:"
    echo "  - sclang failed to start (architecture mismatch or missing deps)"
    echo "  - launcher not built for this arch"
    echo "Inspect /tmp/sclang_post.log, /tmp/sc_launcher_stdin.log, and ~/Library/Logs/Zed/Zed.log for details."
    exit $status
fi
