#!/bin/bash
# Open SuperCollider help in Zed window for word at cursor position
# Uses ZED_FILE, ZED_ROW, ZED_COLUMN environment variables

SCRIPT_DIR="$(dirname "$0")"
SYMBOL=$("$SCRIPT_DIR/word-at-cursor.sh" "$ZED_FILE" "$ZED_ROW" "$ZED_COLUMN" 2>/dev/null)

[ -z "$SYMBOL" ] && { printf 'Class: '; read SYMBOL; }
[ -z "$SYMBOL" ] && exit 1

HELP="/Applications/SuperCollider.app/Contents/Resources/HelpSource/Classes/${SYMBOL}.schelp"

if [ ! -f "$HELP" ]; then
    echo "Not found: $SYMBOL"
    sleep 1
    exit 1
fi

READER="$ZED_WORKTREE_ROOT/tools/schelp/schelp.lua"
OUT="/tmp/${SYMBOL}.md"
pandoc -f "$READER" -t markdown "$HELP" -o "$OUT" && zed "$OUT"
