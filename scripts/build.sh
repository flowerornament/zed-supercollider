#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="release"
WASM_TARGET="wasm32-wasip1"

if [[ "${1:-}" == "--debug" ]]; then
    MODE="debug"
fi

# Build tree-sitter grammar to wasm
echo "Building tree-sitter grammar..."
(
    cd "$ROOT/grammars/supercollider"
    tree-sitter generate
    tree-sitter build --wasm -o ../supercollider.wasm
)
echo "Grammar built: $ROOT/grammars/supercollider.wasm"

echo "Building SuperCollider launcher (${MODE})..."
(
    cd "$ROOT/server/launcher"
    cargo build --${MODE}
)

BIN="$ROOT/server/launcher/target/${MODE}/sc_launcher"

if [[ -x "$BIN" ]]; then
    echo "Built launcher: $BIN"
else
    echo "Build completed but launcher not found at $BIN" >&2
    exit 1
fi

# Ensure the WASM target exists for the Zed extension build.
if ! rustup target list --installed | grep -q "^${WASM_TARGET}$"; then
    echo "Installing Rust target ${WASM_TARGET}..."
    rustup target add "${WASM_TARGET}"
fi

echo "Building Zed extension (${MODE}, target=${WASM_TARGET})..."
(
    cd "$ROOT"
    cargo build --target "${WASM_TARGET}" --${MODE}
)

WASM_ARTIFACT="$ROOT/target/${WASM_TARGET}/${MODE}/zed_supercollider.wasm"
OUT_WASM="$ROOT/extension.wasm"

if [[ -f "$WASM_ARTIFACT" ]]; then
    cp "$WASM_ARTIFACT" "$OUT_WASM"
    echo "Built extension: $OUT_WASM (from $WASM_ARTIFACT)"
else
    echo "Build completed but WASM not found at $WASM_ARTIFACT" >&2
    exit 1
fi

echo "Running launcher tests..."
(
    cd "$ROOT/server/launcher"
    cargo test
)

echo "All builds and tests completed successfully."
