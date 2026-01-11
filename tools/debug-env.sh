#!/bin/bash
# Debug script to show ZED environment variables
# References ZED_SELECTED_TEXT to ensure it's populated

echo "=== ZED Environment Variables ==="
echo "SELECTED_TEXT: $ZED_SELECTED_TEXT"
echo
env | grep ZED | sort
echo
echo "Press enter to close..."
read
