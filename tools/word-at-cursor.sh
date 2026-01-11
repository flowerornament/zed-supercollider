#!/bin/bash
# Extract the word at cursor position from a file
# Usage: word-at-cursor.sh <file> <row> <column>
# Row is 1-indexed (matches ZED_ROW), column is 0-indexed (matches ZED_COLUMN)

FILE="$1"
ROW="$2"
COL="$3"

[ -z "$FILE" ] || [ -z "$ROW" ] || [ -z "$COL" ] && {
    echo "Usage: $0 <file> <row> <column>" >&2
    exit 1
}

# Extract the line (1-indexed row)
LINE=$(sed -n "${ROW}p" "$FILE")

# Use awk to find the word at the column position
echo "$LINE" | awk -v col="$COL" '
{
    # Walk through line finding word boundaries
    start = 0
    in_word = 0
    word_start = 0

    for (i = 1; i <= length($0); i++) {
        c = substr($0, i, 1)
        is_word_char = match(c, /[a-zA-Z0-9_]/)

        if (is_word_char && !in_word) {
            # Starting a new word
            in_word = 1
            word_start = i - 1  # 0-indexed
        } else if (!is_word_char && in_word) {
            # Ending a word
            word_end = i - 1  # 0-indexed, exclusive
            if (col >= word_start && col < word_end) {
                print substr($0, word_start + 1, word_end - word_start)
                exit
            }
            in_word = 0
        }
    }

    # Handle word at end of line
    if (in_word) {
        word_end = length($0)
        if (col >= word_start && col <= word_end) {
            print substr($0, word_start + 1, word_end - word_start)
        }
    }
}
'
