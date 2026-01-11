#!/bin/bash
# Recompile the SuperCollider class library

SC_PORT="${SC_HTTP_PORT:-57130}"
case "$SC_PORT" in ""|0|*[!0-9]*) SC_PORT=57130 ;; esac

export NO_PROXY="${NO_PROXY:-127.0.0.1,localhost}"
HTTP=$(curl --noproxy "*" -fsS -o /dev/null -w "%{http_code}" -X POST "http://127.0.0.1:${SC_PORT}/recompile")
printf "HTTP %s\n" "$HTTP"
