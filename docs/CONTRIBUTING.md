# Repository Guidelines

This repository contains a Zed extension for SuperCollider, providing language support with evaluation capabilities comparable to scnvim. Follow these rules to keep changes consistent and reviewable.

## Key Documents
- `docs/PLAN.md` — Implementation plan with milestones and validation status
- `docs/LOG.md` — Activity log of significant changes
- `docs/USAGE.md` — User quick start guide
- `docs/SETTINGS.md` — Configuration reference
- `docs/TROUBLESHOOTING.md` — Common issues and fixes

## Architecture Overview

The extension uses a **dual-channel architecture**:

1. **LSP Channel** (for intelligence): Zed ↔ stdio ↔ sc_launcher ↔ UDP ↔ sclang/LanguageServer.quark
   - Completions, hover, go-to-definition, diagnostics

2. **HTTP Channel** (for evaluation): Zed Tasks ↔ HTTP ↔ sc_launcher ↔ UDP ↔ sclang
   - Code evaluation, server control (boot/stop/recompile)

This split exists because Zed's extension API cannot programmatically invoke LSP `workspace/executeCommand`. The HTTP channel bypasses this limitation.

## Project Structure

```
extension.toml              # Extension metadata, LSP config
Cargo.toml                  # Rust crate for extension
src/lib.rs                  # Zed extension entry point

languages/SuperCollider/
  config.toml               # Language metadata
  highlights.scm            # Syntax highlighting
  brackets.scm              # Bracket matching
  indents.scm               # Indentation rules
  outline.scm               # Symbol outline
  runnables.scm             # Evaluable block detection (play buttons)

server/launcher/            # Rust binary: LSP bridge + HTTP eval server
  Cargo.toml
  src/main.rs

server/quark/               # Vendored LanguageServer.quark (submodule)
  LanguageServer.quark/

snippets/
  supercollider.json        # Code snippets

docs/                       # User documentation
tests/                      # Test fixtures
```

## Build & Development Commands

```bash
# Install as dev extension
# Zed → Extensions → Install Dev Extension → select repo

# Reload extension after changes
# Cmd+Shift+P → "zed: reload extensions"

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# Build launcher (separate from extension)
cargo build --manifest-path server/launcher/Cargo.toml

# Run launcher probe
cargo run --manifest-path server/launcher/Cargo.toml -- --mode probe

# Test HTTP eval (when implemented)
curl -X POST -d "1 + 1" http://localhost:57130/eval
```

## Coding Style

- Rust 2021 edition, 4-space indentation, ~100 char lines
- Names: `snake_case` modules, `PascalCase` types, `SCREAMING_SNAKE_CASE` constants
- Tree-sitter queries: small, composable, precise captures
- Keep extension ID stable; language dir is `languages/SuperCollider`

## Key Files to Understand

| File | Purpose |
|------|---------|
| `src/lib.rs` | Extension entry, LSP registration |
| `server/launcher/src/main.rs` | LSP bridge, HTTP server |
| `languages/SuperCollider/runnables.scm` | Detects evaluable blocks |
| `server/quark/.../ExecuteCommandProvider.sc` | Handles eval commands |

## Runnables System

Play buttons in the gutter are powered by `runnables.scm`:

```scheme
((code_block) @code @run
  (#set! tag sc-eval))
```

- `@code` → captured text available as `$ZED_CUSTOM_code` in tasks
- `@run` → where play button appears
- `#set! tag sc-eval` → matches tasks with `"tags": ["sc-eval"]`

## SuperCollider Semantics

- **Client vs Server**: `sclang` (interpreter) talks to `scsynth` (audio server) via OSC
- **Boot**: `Server.default.boot` or `s.boot`
- **Hard Stop**: `CmdPeriod.run` — stops all synths
- **Recompile**: `thisProcess.recompile` — rebuilds class library

## Commit Guidelines

- Conventional Commits with scopes: `feat(language):`, `fix(server):`, `docs:`
- PRs include: problem/solution summary, validation steps, before/after behavior
- CI must pass; formatter and clippy clean

## Testing

- Smoke test: open `.scd` → click play button → verify output in terminal
- Unit tests: `cargo test`
- Fixtures: `tests/fixtures/`

## Security Notes

- Launch external processes only via designated launcher
- Document all env vars and paths in `docs/SETTINGS.md`
- Avoid writes outside extension storage
