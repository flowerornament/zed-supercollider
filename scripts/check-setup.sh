#!/usr/bin/env bash
# If this script fails in a sandboxed environment, rerun with elevated permissions
# so it can access the system sclang (e.g. /Applications/SuperCollider.app/...).
set -euo pipefail

# Colors (if terminal supports it)
if [[ -t 1 ]] && command -v tput >/dev/null 2>&1; then
    GREEN=$(tput setaf 2)
    RED=$(tput setaf 1)
    YELLOW=$(tput setaf 3)
    BOLD=$(tput bold)
    RESET=$(tput sgr0)
else
    GREEN="" RED="" YELLOW="" BOLD="" RESET=""
fi

pass() { echo "${GREEN}✓${RESET} $1"; }
fail() { echo "${RED}✗${RESET} $1"; }
warn() { echo "${YELLOW}!${RESET} $1"; }
header() { echo; echo "${BOLD}$1${RESET}"; echo "─────────────────────────────────"; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEFAULT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ROOT="${ZED_WORKTREE_ROOT:-$DEFAULT_ROOT}"
DEV_LAUNCHER="$ROOT/server/launcher/target/release/sc_launcher"
LAUNCHER="${SC_LAUNCHER_PATH:-}"

header "Launcher"

if [[ -z "$LAUNCHER" && -x "$DEV_LAUNCHER" ]]; then
    LAUNCHER="$DEV_LAUNCHER"
    pass "Using dev build: $LAUNCHER"
elif [[ -z "$LAUNCHER" ]]; then
    LAUNCHER="$(command -v sc_launcher 2>/dev/null || true)"
    if [[ -n "$LAUNCHER" ]]; then
        pass "Using PATH: $LAUNCHER"
    fi
fi

if [[ -z "$LAUNCHER" ]]; then
    fail "Launcher not found"
    echo "  Build it: cd server/launcher && cargo build --release"
    echo "  Or set SC_LAUNCHER_PATH environment variable"
    exit 1
fi

if command -v file >/dev/null 2>&1; then
    ARCH_INFO=$(file -b "$LAUNCHER" 2>/dev/null || true)
    echo "  Architecture: $ARCH_INFO"
fi

header "SuperCollider"

set +e
OUTPUT=$("$LAUNCHER" --mode probe 2>&1)
status=$?
set -e

if [[ $status -eq 0 ]]; then
    pass "Probe successful"

    # Parse and display JSON output nicely
    if command -v python3 >/dev/null 2>&1; then
        SCLANG_PATH=$(echo "$OUTPUT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('sclang',{}).get('path',''))" 2>/dev/null || true)
        SCLANG_VERSION=$(echo "$OUTPUT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('sclang',{}).get('version',''))" 2>/dev/null || true)

        if [[ -n "$SCLANG_PATH" ]]; then
            echo "  sclang: $SCLANG_PATH"
        fi
        if [[ -n "$SCLANG_VERSION" ]]; then
            echo "  Version: $SCLANG_VERSION"
        fi
    else
        echo "$OUTPUT"
    fi
else
    fail "Probe failed (exit $status)"
    echo ""
    echo "Common causes:"
    echo "  • sclang failed to start (architecture mismatch or missing deps)"
    echo "  • Launcher not built for this architecture"
    echo ""
    echo "Troubleshooting:"
    echo "  • Check /tmp/sclang_post.log"
    echo "  • Check /tmp/sc_launcher_stdin.log"
    echo "  • Check ~/Library/Logs/Zed/Zed.log"
    exit $status
fi

header "Summary"
pass "Setup OK - ready to use SuperCollider in Zed"
