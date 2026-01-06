#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIG_PATH="$ROOT/languages/SuperCollider/config.toml"

if [[ ! -f "$CONFIG_PATH" ]]; then
    echo "config file not found at $CONFIG_PATH" >&2
    exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
    echo "python3 is required to validate the language config" >&2
    exit 1
fi

python3 - <<'PY' "$CONFIG_PATH"
import sys
from pathlib import Path

config_path = Path(sys.argv[1])

try:
    import tomllib  # Python 3.11+
except ImportError:  # pragma: no cover
    try:
        import tomli as tomllib  # type: ignore
    except ImportError as exc:  # pragma: no cover
        raise SystemExit("tomllib (or tomli) is required for validation") from exc

raw = config_path.read_bytes()
data = tomllib.loads(raw.decode())

errors = []

required = ["name", "grammar", "path_suffixes", "line_comments", "tab_size", "hard_tabs"]
banned = ["opt_into_language_servers", "scope_opt_in_language_servers"]

for key in required:
    if key not in data:
        errors.append(f"missing required key `{key}`")

for key in banned:
    if key in data:
        errors.append(f"banned key `{key}` present")

suffixes = data.get("path_suffixes")
if not isinstance(suffixes, list) or not all(isinstance(s, str) and s for s in suffixes):
    errors.append("path_suffixes must be a non-empty array of strings")

word_chars = data.get("word_characters")
if word_chars is not None:
    if not isinstance(word_chars, list) or not all(isinstance(c, str) for c in word_chars):
        errors.append("word_characters must be an array of strings when present")
    else:
        too_long = [c for c in word_chars if len(c) != 1]
        if too_long:
            joined = ", ".join(repr(c) for c in too_long)
            errors.append(f"word_characters entries must be single characters (invalid: {joined})")

if errors:
    print("config validation failed:")
    for err in errors:
        print(f"- {err}")
    sys.exit(1)

print(f"{config_path}: ok")
PY
