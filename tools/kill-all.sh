#!/bin/bash
# Emergency cleanup: kill launcher and sclang processes
# To restart, use "Restart All Servers" from the Zed command palette

PID_FILE="${SC_TMP_DIR:-${TMPDIR:-/tmp}}/sc_launcher.pid"
[ ! -f "$PID_FILE" ] && PID_FILE="/tmp/sc_launcher.pid"

if [ ! -f "$PID_FILE" ]; then
    echo "No PID file found; nothing to stop"
    exit 0
fi

echo "Using: $PID_FILE"

LAUNCHER_PID=$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(d.get("launcher_pid",""))' "$PID_FILE" 2>/dev/null)
SCLANG_PID=$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(d.get("sclang_pid",""))' "$PID_FILE" 2>/dev/null)

if [ -n "$SCLANG_PID" ] && ps -p "$SCLANG_PID" >/dev/null 2>&1; then
    echo "Stopping sclang (pid $SCLANG_PID)"
    kill "$SCLANG_PID" 2>/dev/null || true
fi

# Kill any scsynth processes (spawned by sclang)
pkill -f scsynth 2>/dev/null && echo "Stopped scsynth processes" || true

if [ -n "$LAUNCHER_PID" ] && ps -p "$LAUNCHER_PID" >/dev/null 2>&1; then
    echo "Stopping launcher (pid $LAUNCHER_PID)"
    kill "$LAUNCHER_PID" 2>/dev/null || true
fi

rm -f "$PID_FILE"
echo ""
echo "Done. To restart, use 'Restart All Servers' from the Zed command palette."
