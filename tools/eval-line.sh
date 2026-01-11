#!/bin/bash
# Evaluate the current line at cursor position
# Uses ZED_FILE, ZED_ROW environment variables

SC_PORT="${SC_HTTP_PORT:-57130}"
case "$SC_PORT" in ""|0|*[!0-9]*) SC_PORT=57130 ;; esac

[ -z "$ZED_FILE" ] || [ -z "$ZED_ROW" ] && {
    echo "Missing ZED_FILE or ZED_ROW" >&2
    exit 1
}

CODE=$(sed -n "${ZED_ROW}p" "$ZED_FILE")

[ -z "$CODE" ] && {
    echo "Empty line at row $ZED_ROW" >&2
    exit 1
}

# Save for debugging
printf "%s" "$CODE" > /tmp/sc_eval_last.txt

export NO_PROXY="${NO_PROXY:-127.0.0.1,localhost}"
HTTP=$(printf "%s" "$CODE" | curl --noproxy "*" -fsS -o /dev/null -w "%{http_code}" \
    -H "Content-Type: text/plain" --data-binary @- "http://127.0.0.1:${SC_PORT}/eval")

printf "HTTP %s\n" "$HTTP"
