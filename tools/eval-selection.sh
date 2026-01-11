#!/bin/bash
# Evaluate selected text
# Debug version with logging

SC_PORT="${SC_HTTP_PORT:-57130}"
case "$SC_PORT" in ""|0|*[!0-9]*) SC_PORT=57130 ;; esac

LOG="/tmp/sc_eval_selection.log"

{
    echo "=== $(date) ==="
    echo "ZED_SELECTED_TEXT length: ${#ZED_SELECTED_TEXT}"
    echo "ZED_SELECTED_TEXT value:"
    echo "---START---"
    printf "%s" "$ZED_SELECTED_TEXT"
    echo ""
    echo "---END---"
} >> "$LOG"

# Check if selection is empty
if [ -z "$ZED_SELECTED_TEXT" ]; then
    echo "ERROR: No text selected" >> "$LOG"
    exit 1
fi

# Save for debugging
printf "%s" "$ZED_SELECTED_TEXT" > /tmp/sc_eval_last.txt

# Send to SC
export NO_PROXY="${NO_PROXY:-127.0.0.1,localhost}"
RESPONSE=$(printf "%s" "$ZED_SELECTED_TEXT" | curl --noproxy "*" -sS -X POST \
    -H "Content-Type: text/plain" \
    --data-binary @- \
    "http://127.0.0.1:${SC_PORT}/eval" 2>&1)

CURL_EXIT=$?

{
    echo "curl exit code: $CURL_EXIT"
    echo "Response: $RESPONSE"
    echo ""
} >> "$LOG"

if [ $CURL_EXIT -ne 0 ]; then
    echo "ERROR: curl failed with exit $CURL_EXIT" >&2
    exit 1
fi

echo "$RESPONSE"
