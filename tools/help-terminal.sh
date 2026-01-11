#!/bin/bash
# Show SuperCollider help for word at cursor position
# Uses ZED_FILE, ZED_ROW, ZED_COLUMN environment variables

SCRIPT_DIR="$(dirname "$0")"
SYMBOL=$("$SCRIPT_DIR/word-at-cursor.sh" "$ZED_FILE" "$ZED_ROW" "$ZED_COLUMN" 2>/dev/null)

[ -z "$SYMBOL" ] && { printf 'Class: '; read SYMBOL; }
[ -z "$SYMBOL" ] && exit 1

HELP="/Applications/SuperCollider.app/Contents/Resources/HelpSource/Classes/${SYMBOL}.schelp"

if [ -f "$HELP" ]; then
    READER="$ZED_WORKTREE_ROOT/tools/schelp/schelp.lua"
    if [ -f "$READER" ] && command -v pandoc >/dev/null && command -v glow >/dev/null; then
        pandoc -f "$READER" -t markdown "$HELP" | glow -p
    elif [ -f "$READER" ] && command -v pandoc >/dev/null; then
        pandoc -f "$READER" -t markdown "$HELP" | less
    else
        less "$HELP"
    fi
else
    echo "Not found: $SYMBOL"
    read
fi
