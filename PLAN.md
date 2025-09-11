# Implementation Plan — Zed SuperCollider Extension

This plan turns the migration from scnvim into concrete, verifiable work. Milestones are incremental, each with clear tasks, files to touch, validation commands, and acceptance criteria.

## Objectives
- Achieve day‑to‑day parity with scnvim for evaluation, post output, start/stop/recompile, help, LSP features, and snippets.
- Ship a maintainable Rust→Wasm Zed extension with Tree‑sitter queries and a reliable LSP bridge.

## Scope & Non‑Goals
- In scope: SC language integration, LSP bootstrap, evaluation, post output, help, snippets, keymaps, docs.
- Non‑goals v1: Custom UI panes, complex status widgets, non‑LSP refactors of LanguageServer.quark.

## Architecture Summary (ADR‑001)
- Language intelligence via LSP (LanguageServer.quark) through a small stdio bridge.
- Syntax/structure via Tree‑sitter (highlight, indents, outline, runnables).
- Evaluation primarily over LSP; terminal task as fallback post window.
- Extension in Rust using `zed_extension_api`.

## Milestone M1 — Language Skeleton
Tasks
- Create `extension.toml`, `Cargo.toml`, `src/lib.rs` with `zed::register_extension!`.
- Pin grammar in `extension.toml`:
  - `[grammars.supercollider] repository = "https://github.com/madskjeldgaard/tree-sitter-supercollider" ; rev = "<pin>"`
- Add `languages/SuperCollider/config.toml` mapping `sc`/`scd`.
- Author minimal queries: `highlights.scm`, `brackets.scm`, `indents.scm`, `outline.scm` (optional: `injections.scm`, `textobjects.scm`).
Files
- `extension.toml`, `Cargo.toml`, `src/lib.rs`
- `languages/SuperCollider/{config.toml,highlights.scm,brackets.scm,indents.scm,outline.scm}`
Validation
- Install as Dev Extension in Zed; open `.scd` file; verify highlighting, bracket matching, outline symbols.
Acceptance
- SC files load with grammar + basic structure features.

## Milestone M2 — LSP Bootstrap
Tasks
- Scaffold `server/launcher` (Rust): detect `sclang`, ensure LanguageServer.quark installed, start LSP, bridge transport to stdio.
- Wire LSP in `src/lib.rs` with `zed_extension_api::lsp` for “SuperCollider”.
- Settings: `supercollider.sclangPath`, `supercollider.confYamlPath?`, health check command.
 - Workspace: add Cargo workspace including `server/launcher` for unified builds.
Files
- `server/launcher/{Cargo.toml,src/main.rs}`
- `src/lib.rs` (LSP registration), `extension.toml` (settings schema)
Validation
- Open `.scd`; check completion, hover, go‑to‑definition, diagnostics.
- Run “Check setup” command; confirm probes succeed.
Acceptance
- LSP features function on typical SC files.

## Milestone M3 — Evaluate & Post Window
Tasks
- Add client commands: `eval_line`, `eval_selection`, `eval_block` (Tree‑sitter region logic).
- Send code via `workspace/executeCommand` to LSP with command ids:
  - `supercollider.evalLine|evalSelection|evalBlock|hardStop|recompile|bootServer|quitServer`
- Server side: evaluate via `Interpreter.interpret`, stream output as `window/logMessage`.
  - Map `hardStop` to `CmdPeriod.run` (stop all nodes on the server).
  - Map `recompile` to `thisProcess.recompile` (rebuild class library).
  - Map `bootServer` to `s.boot`; `quitServer` to `s.quit`.
- Fallback: provide a documented Task snippet for users to run a persistent `sclang` terminal via Zed’s Tasks panel; optionally add an extension command to spawn `sclang` in a terminal if the API supports it.
- Keybindings in `extension.toml` mirroring scnvim defaults where reasonable.
Files
- `src/lib.rs` (commands), `server/launcher/src/main.rs` (handlers)
- `tasks/sc-post.ztask.json`, `extension.toml` (keymaps)
Validation
- “Eval Selection/Line/Block” works; post output visible (LSP or terminal fallback).
- “Hard Stop” stops sound; “Recompile” restores completions.
Acceptance
- Interactive evaluation round‑trip works reliably; post output usable.

## Milestone M4 — Help & Docs
Tasks
- Implement `open_help` via LSP request returning Markdown; open in new buffer.
- Optional converter path: `.schelp` → Markdown via external tool (e.g., `pandoc`).
- Enhance hover to show signatures and short docs (via LSP).
Files
- `src/lib.rs` (help command), `extension.toml` (settings), launcher server side if needed.
Validation
- Hover over `SinOsc` shows signature excerpt.
- “Open Help” displays docs in a buffer; converter path renders when set.
Acceptance
- Help workflows usable from editor for classes and methods.

## Milestone M5 — Snippets & Polish
Tasks
- Create `snippets/supercollider.json` (SynthDef, UGen, Pattern templates).
- Finalize keymaps; document overrides in README.
- Author `docs/MIGRATION.md` (Neovim→Zed tips) and `docs/TROUBLESHOOTING.md` (ports, SCIDE conflicts, conf YAML).
Files
- `snippets/supercollider.json`, `docs/{MIGRATION.md,TROUBLESHOOTING.md}`
Validation
- Snippets insert correctly; docs reflect actual behavior and settings.
Acceptance
- Day‑to‑day authoring is fast; docs cover setup and pitfalls.

## Cross‑Cutting
- CI: add `wasm32-wasi` build and basic tests (`cargo test`).
- Tests: unit tests for region selection and command plumbing; E2E fixtures in `tests/fixtures/`.
- Reference parity: use `docs/reference/scnvim/{UPSTREAM_README.md,SCNvim.txt}` to map features and verify semantics.

## Risk Management
- LSP transport mismatch: normalize via launcher (stdio). Long‑term: PR to Quark for stdio mode.
- Streaming/latency: prefer LSP `logMessage`; keep terminal fallback.
- SCIDE conflicts: document in Troubleshooting; allow dedicated `sclang_conf.yaml`.

## PR Sequence (suggested)
1) M1 skeleton + queries
2) LSP launcher + registration + settings
3) Eval commands + post window + fallback task + keymaps
4) Help request + optional converter path
5) Snippets + docs + polish

## Review Checklist (per PR)
- Builds for `wasm32-wasi`; `cargo fmt` and `clippy` clean
- Tests/fixtures updated; manual smoke test recorded
- Docs and settings updated; CHANGELOG entry if needed

## Definition of Done
- All milestones M1–M5 accepted
- Parity checklist satisfied (evaluation, post, start/stop/recompile, help, LSP features, snippets)
- Docs complete; troubleshooting covers common issues; reference snapshots pinned
