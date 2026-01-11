#!/bin/bash
# Open SuperCollider help in browser for word at cursor position
# Uses ZED_FILE, ZED_ROW, ZED_COLUMN environment variables

SCRIPT_DIR="$(dirname "$0")"
SYMBOL=$("$SCRIPT_DIR/word-at-cursor.sh" "$ZED_FILE" "$ZED_ROW" "$ZED_COLUMN" 2>/dev/null)

[ -z "$SYMBOL" ] && { printf 'Class: '; read SYMBOL; }
[ -z "$SYMBOL" ] && exit 1

echo "Opening docs for: $SYMBOL"
open "https://docs.supercollider.online/Classes/$SYMBOL"
