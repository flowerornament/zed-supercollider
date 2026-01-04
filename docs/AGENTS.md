# Repository Guidelines

This repository contains a Zed extension for SuperCollider, migrating scnvim features to Zed. Follow these concise rules to keep changes consistent and reviewable. See `docs/MIGRATION_PLAN.md` (source plan) and `PLAN.md` (implementation plan) for details.

## Project Structure & Module Organization
- `extension.toml` — extension metadata, commands, settings, LSP mapping.
- `src/lib.rs` — Rust entry implementing `zed::Extension`.
- `languages/SuperCollider/` — `config.toml` and Tree‑sitter queries: `highlights.scm`, `indents.scm`, `brackets.scm`, `outline.scm` (optional: `injections.scm`, `textobjects.scm`).
- `snippets/` — `supercollider.json` snippet definitions.
- `server/launcher/` — helper to start `sclang` + LanguageServer.quark (LSP bridge).
- `server/quark/` — optional vendored LanguageServer.quark (submodule or downloader).
- `tasks/` — optional dev helpers; fallback post window is provided via user Tasks snippets documented under `docs/`.
- `docs/` — `MIGRATION.md`, `MIGRATION_PLAN.md`, `SETTINGS.md`, `TROUBLESHOOTING.md`, user guides.
- `tests/` — unit/E2E tests; fixtures under `tests/fixtures/`.

## Build, Test, and Development Commands
- Run locally: Zed → Extensions → Install Dev Extension → select repo. Reload via “Zed: Reload Extensions”.
- Format/Lint: `cargo fmt --all` ; `cargo clippy --all-targets --all-features -- -D warnings`
- Unit tests: `cargo test`
- Smoke test: open a `.scd` → run “Eval Selection” → confirm output in the post/log panel (or use the user Task snippet from docs to run a persistent `sclang` terminal).
- Build launcher only: `cargo build -p sc_launcher`
- Run setup probe: `cargo run -p sc_launcher -- --sclang-path $(which sclang)`

## Coding Style & Naming Conventions
- Rust 2021; 4‑space indentation; target line length ≈ 100.
- Names: modules `snake_case`, types/traits `PascalCase`, constants `SCREAMING_SNAKE_CASE`.
- Keep Tree‑sitter queries small and composable; prefer precise captures over broad ones.
- Keep the extension id stable; language dir is `languages/SuperCollider`.

## Testing Guidelines
- Isolate logic behind traits; unit‑test without editor hooks when possible.
- Place E2E fixtures in `tests/fixtures/`; verify evaluation, help lookup, and hard‑stop paths.
- Aim for high coverage on core logic (>80% ideal) without blocking on UI glue.

## Commit & Pull Request Guidelines
- Conventional Commits with scopes: `language`, `server`, `snippets`, `tasks`.
- PRs include: problem/solution summary, linked issues, before/after behavior, logs/screenshots for UX changes.
- CI must pass; formatter and clippy clean.

## Security & Configuration Tips
- Launch external processes only via the designated launcher; document env vars and paths.
- Avoid writing outside extension storage; minimize network calls and document them in `docs/TROUBLESHOOTING.md`.
- Expose user‑tunable settings in `extension.toml` and mirror them in docs.

## Migration from scnvim
- Feature parity goals: evaluation (line/selection/block), post window, start/stop server + recompile class library, help, completion/hover/defs, snippets.
- Prefer LSP for intelligence and evaluation; fallback to terminal task for post window.
- Keep command ids and language ids stable; document any breaking changes in `docs/MIGRATION.md`.
- Validate parity via smoke tests and `tests/fixtures/`; document keybindings/workflows in `README.md`.

Note on semantics
- Client vs Server: `sclang` (client/interpreter) talks to `scsynth` (audio server) via OSC. Boot with `s.boot`, stop audio with CmdPeriod (aka hard stop), and quit the server with `s.quit`.
- Recompile: use `thisProcess.recompile` to rebuild the class library from within `sclang`.

## Agent‑Specific Instructions
- Keep diffs focused (<400 LOC when practical) and touch only relevant subtrees.
- Update `extension.toml` and related queries together; run the smoke test on sample `.sc`/`.scd` files before opening a PR.
