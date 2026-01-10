#!/bin/bash
# Test harness for schelp-to-markdown converter
# Usage: ./run_tests.sh [schelp_file]

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
READER="$SCRIPT_DIR/../schelp.lua"

if [ ! -f "$READER" ]; then
    echo "Error: schelp.lua not found at $READER"
    exit 1
fi

# Check for pandoc
if ! command -v pandoc &> /dev/null; then
    echo "Error: pandoc is required but not installed"
    echo "Install with: brew install pandoc"
    exit 1
fi

convert_file() {
    local input="$1"
    local output="${input%.schelp}.md"

    echo "Converting: $input"
    if pandoc -f "$READER" -t markdown "$input" -o "$output" 2>&1; then
        echo "  -> $output"
        return 0
    else
        echo "  FAILED"
        return 1
    fi
}

# If specific file provided, convert just that
if [ -n "$1" ]; then
    if [ -f "$1" ]; then
        convert_file "$1"
    else
        echo "File not found: $1"
        exit 1
    fi
else
    # Convert all .schelp files in test directory
    echo "Converting all .schelp files in $SCRIPT_DIR"
    echo ""

    failed=0
    total=0

    for schelp in "$SCRIPT_DIR"/*.schelp; do
        if [ -f "$schelp" ]; then
            total=$((total + 1))
            if ! convert_file "$schelp"; then
                failed=$((failed + 1))
            fi
        fi
    done

    echo ""
    echo "Results: $((total - failed))/$total passed"

    if [ $failed -gt 0 ]; then
        exit 1
    fi
fi
