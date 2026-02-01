# Format code
fmt:
    cd server/launcher && cargo fmt

# Run clippy
lint:
    cd server/launcher && cargo clippy --all-targets

# Run clippy strict (warnings as errors)
lint-strict:
    cd server/launcher && cargo clippy --all-targets -- -D warnings

# Run tests
test:
    cd server/launcher && cargo test

# Run all checks (fmt + lint + test)
check:
    cd server/launcher && cargo fmt --check
    cd server/launcher && cargo clippy --all-targets
    cd server/launcher && cargo test

# Full build (grammar + launcher + extension)
build:
    ./scripts/build.sh

# Clean build artifacts
clean:
    cargo clean
    rm -rf target/wasm32-wasip1
