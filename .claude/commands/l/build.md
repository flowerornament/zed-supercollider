---
description: Build the Zed extension (grammar + launcher + wasm)
---

Build the zed-supercollider extension.

## Steps

1. Run the build:
   ```bash
   just build
   ```

2. Check for errors in output. Common issues:
   - Missing emscripten → `brew install emscripten`
   - Grammar generation failed → check `grammars/supercollider/`
   - Rust wasm build failed → check Cargo.toml dependencies

3. If build succeeds, remind user to reload extension in Zed:
   - `Cmd+Shift+P` → "zed: reload extensions"

## What it builds

- `grammars/supercollider.wasm` - tree-sitter grammar
- `target/wasm32-wasip1/release/` - extension wasm
- Launcher binary (if not already built)
