#!/bin/bash
# Evaluate the code block containing the cursor
# Finds enclosing () block and evaluates it
# Uses ZED_FILE, ZED_ROW environment variables

SC_PORT="${SC_HTTP_PORT:-57130}"
case "$SC_PORT" in ""|0|*[!0-9]*) SC_PORT=57130 ;; esac

[ -z "$ZED_FILE" ] || [ -z "$ZED_ROW" ] && {
    echo "Missing ZED_FILE or ZED_ROW" >&2
    exit 1
}

# Find enclosing block using awk
# Searches backward for ( and forward for matching )
CODE=$(awk -v cursor_line="$ZED_ROW" '
BEGIN {
    depth = 0
    block_start = 0
    block_end = 0
    in_block = 0
}
{
    lines[NR] = $0

    # Track paren depth
    for (i = 1; i <= length($0); i++) {
        c = substr($0, i, 1)
        if (c == "(") {
            if (depth == 0) {
                potential_start = NR
            }
            depth++
        } else if (c == ")") {
            depth--
            if (depth == 0) {
                # Found a complete block
                if (potential_start <= cursor_line && NR >= cursor_line) {
                    block_start = potential_start
                    block_end = NR
                }
            }
        }
    }
}
END {
    if (block_start > 0 && block_end > 0) {
        for (i = block_start; i <= block_end; i++) {
            print lines[i]
        }
    }
}
' "$ZED_FILE")

[ -z "$CODE" ] && {
    echo "No enclosing () block found at row $ZED_ROW" >&2
    exit 1
}

# Save for debugging
printf "%s" "$CODE" > /tmp/sc_eval_last.txt

export NO_PROXY="${NO_PROXY:-127.0.0.1,localhost}"
HTTP=$(printf "%s" "$CODE" | curl --noproxy "*" -fsS -o /dev/null -w "%{http_code}" \
    -H "Content-Type: text/plain" --data-binary @- "http://127.0.0.1:${SC_PORT}/eval")

printf "HTTP %s\n" "$HTTP"
