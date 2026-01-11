#!/bin/bash
# Evaluate code passed via ZED_CUSTOM_CODE (from play button / runnables)
#
# This task is triggered by the ▶ play button in the editor gutter.
# Zed's runnables system (runnables.scm) detects code_block nodes and
# passes their content here via ZED_CUSTOM_CODE.

SC_PORT="${SC_HTTP_PORT:-57130}"
case "$SC_PORT" in ""|0|*[!0-9]*) SC_PORT=57130 ;; esac

# ZED_CUSTOM_CODE comes from runnables.scm @code capture
CODE="${ZED_CUSTOM_CODE:-}"

[ -z "$CODE" ] && exit 0

# Save for debugging
printf "%s" "$CODE" > /tmp/sc_eval_last.txt

export NO_PROXY="${NO_PROXY:-127.0.0.1,localhost}"
HTTP=$(printf "%s" "$CODE" | curl --noproxy "*" -fsS -o /dev/null -w "%{http_code}" \
    -H "Content-Type: text/plain" --data-binary @- "http://127.0.0.1:${SC_PORT}/eval")

printf "HTTP %s\n" "$HTTP"
