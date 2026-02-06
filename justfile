# Zed SuperCollider Extension
# Run `just --list` to see all available commands

set shell := ["bash", "-euo", "pipefail", "-c"]

wasm_target := "wasm32-wasip1"
root := justfile_directory()

# List available commands
default:
    @just --list

# Full release build (grammar + launcher + extension + tests)
build: (_build "release")

# Full debug build
build-debug: (_build "debug")

# Internal build recipe
_build mode:
    #!/usr/bin/env bash
    set -euo pipefail

    echo "=== Building tree-sitter grammar ==="
    (
        cd "{{root}}/grammars/supercollider"
        tree-sitter generate
        tree-sitter build --wasm -o ../supercollider.wasm
    )
    echo "Grammar built: {{root}}/grammars/supercollider.wasm"

    echo ""
    echo "=== Building launcher ({{mode}}) ==="
    cargo build --manifest-path "{{root}}/server/launcher/Cargo.toml" --{{mode}}

    bin="{{root}}/server/launcher/target/{{mode}}/sc_launcher"
    if [[ -x "$bin" ]]; then
        echo "Built: $bin"
    else
        echo "ERROR: Launcher not found at $bin" >&2
        exit 1
    fi

    echo ""
    echo "=== Building extension ({{mode}}) ==="
    if ! rustup target list --installed | grep -q "^{{wasm_target}}$"; then
        echo "Installing {{wasm_target}}..."
        rustup target add "{{wasm_target}}"
    fi

    cargo build --manifest-path "{{root}}/Cargo.toml" --target "{{wasm_target}}" --{{mode}}

    wasm_src="{{root}}/target/{{wasm_target}}/{{mode}}/zed_supercollider.wasm"
    wasm_dst="{{root}}/extension.wasm"
    if [[ -f "$wasm_src" ]]; then
        cp "$wasm_src" "$wasm_dst"
        echo "Built: $wasm_dst"
    else
        echo "ERROR: WASM not found at $wasm_src" >&2
        exit 1
    fi

    echo ""
    echo "=== Running tests ==="
    cargo test --manifest-path "{{root}}/server/launcher/Cargo.toml"

    echo ""
    echo "Build complete."

# Run all quality checks (fmt + lint + test)
check:
    @echo "=== Format check ==="
    cargo fmt --manifest-path "{{root}}/server/launcher/Cargo.toml" --check
    @echo ""
    @echo "=== Lint ==="
    cargo clippy --manifest-path "{{root}}/server/launcher/Cargo.toml" --all-targets
    @echo ""
    @echo "=== Tests ==="
    cargo test --manifest-path "{{root}}/server/launcher/Cargo.toml"
    @echo ""
    @echo "All checks passed."

# Format code
fmt:
    cargo fmt --manifest-path "{{root}}/server/launcher/Cargo.toml"

# Run clippy
lint:
    cargo clippy --manifest-path "{{root}}/server/launcher/Cargo.toml" --all-targets

# Run clippy with warnings as errors
lint-strict:
    cargo clippy --manifest-path "{{root}}/server/launcher/Cargo.toml" --all-targets -- -D warnings

# Run tests
test:
    cargo test --manifest-path "{{root}}/server/launcher/Cargo.toml"

# Clean all build artifacts
clean:
    cargo clean --manifest-path "{{root}}/Cargo.toml"
    cargo clean --manifest-path "{{root}}/server/launcher/Cargo.toml"
    rm -f "{{root}}/extension.wasm"
    rm -f "{{root}}/grammars/supercollider.wasm"
