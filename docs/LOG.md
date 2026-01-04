# Activity Log

This log captures high-level actions taken by the agent for transparency and traceability.

- 2025-09-11 — Added `AGENTS.md`
  - Created initial contributor guide tailored for a Zed SuperCollider extension (structure, build/test, style, testing, PR guidelines).

- 2025-09-11 — Merged scnvim migration guidance into `AGENTS.md`
  - Rewrote to align with Rust + Tree-sitter layout; added sections for launcher, tasks, docs, and migration parity goals.
  - Linked to a forthcoming detailed migration plan.

- 2025-09-11 — Added `docs/MIGRATION_PLAN.md`
  - Captured the full Navigator plan (ADR-001, milestones, risks, prompts, acceptance criteria) separate from the concise AGENTS guide.

- 2025-09-11 — Vendored scnvim reference docs
  - Created `docs/reference/scnvim/` with `README.md`, `SOURCES.md`, `NOTICE`.
  - Pulled upstream snapshot (for contributor reference only):
    - `UPSTREAM_README.md` (from scnvim README)
    - `SCNvim.txt` (help doc)
    - `LICENSE` (GPL-3.0)
  - Pin details: repo `github.com/davidgranstrom/scnvim`, commit `8148e9b5700956b14b0202ee4b08d6856510d3fd`, license `GPL-3.0`.

- 2025-09-11 — Added `PLAN.md`
  - Step-by-step implementation plan with milestones (M1-M5), tasks, files to touch, validation, acceptance, risks, and PR sequence.

- 2025-09-11 — Reviewed official SuperCollider docs; updated semantics
  - Fetched SC docs on client/server, `CmdPeriod`, `Server`, and `ThisProcess`.
  - Updated `AGENTS.md`, `PLAN.md`, and `docs/MIGRATION_PLAN.md` to clarify:
    - Client (`sclang`) vs Server (`scsynth`) via OSC.
    - `hardStop` maps to `CmdPeriod.run`; `recompile` maps to `thisProcess.recompile`.
    - Boot/quit server via `s.boot`/`s.quit`.

- 2025-09-11 — Scaffolded M1 skeleton
  - Added `extension.toml` (grammar pin placeholder), `Cargo.toml`, and `src/lib.rs` with minimal `zed_extension_api` extension.
  - Added language config `languages/SuperCollider/config.toml` and minimal queries (`highlights.scm`, `brackets.scm`, `indents.scm`, `outline.scm`).
  - Added starter snippets at `snippets/supercollider.json` (SynthDef, Pbind).

- 2025-09-11 — Read Zed extension docs; aligned plan
  - Confirmed Rust cdylib + `register_extension!` pattern and using latest `zed_extension_api` compatible with target Zed versions.
  - Adjusted fallback post window approach: provide user Task snippet in docs instead of bundling a `.ztask.json` file.

- 2025-09-11 — Initialized git repository and committed scaffold
  - Added `.gitignore` and removed cached artifacts; first commit with scaffold and docs.

- 2025-09-11 — M2 kick-off: LSP launcher stub
  - Added `server/launcher` Rust bin crate. Currently probes `sclang -v`; to be extended with Quark install and stdio bridge.

- 2025-09-11 — Pinned grammar and added settings/docs
  - Pinned `tree-sitter-supercollider` commit in `extension.toml`.
  - Added `docs/SETTINGS.md`, `docs/MIGRATION.md`, `docs/TROUBLESHOOTING.md`, `docs/TASKS_SNIPPET.md`.

- 2025-09-11 — Fixed manifest for Zed install
  - Updated `extension.toml` to use `id` and `name` fields (removed `display_name`), and normalized authors format.

- 2025-09-11 — Fixed dev build hang in Zed
  - Removed Cargo workspace from extension root; build launcher with `cargo build --manifest-path server/launcher/Cargo.toml`.

- 2025-09-11 — Language loads; added initial highlights and outline
  - Fixed query node names to match grammar (comments, strings, numbers; class and method names in outline).

- 2025-09-11 — Language directory normalization for Zed
  - Restored canonical `languages/SuperCollider/` directory (matching language name) and removed duplicates to avoid case-insensitive FS ambiguity.

- 2025-10-16 — Implemented stdio-UDP LanguageServer bridge, manifest wiring, and default settings
  - `sc_launcher --mode lsp` now spawns `sclang --daemon`, forwards LSP traffic over UDP, streams logs, and shuts down cleanly when stdin closes.
  - Registered the SuperCollider language server in `extension.toml` and provided default initialization/workspace settings from `src/lib.rs`.
  - Discovered Zed extension API cannot emit LSP executeCommand/evaluateSelection, so downstream eval commands must be surfaced via Code Actions/CodeLens rather than extension-driven commands.
  - Updated launcher docs, troubleshooting, and `docs/SETTINGS.md` to explain the new setup, required arguments, and configurable evaluation/logging knobs.

- 2025-10-16 — Submodule vendored LanguageServer.quark & launcher vendored-path support
  - Added fork `flowerornament/LanguageServer.quark` as `server/quark/LanguageServer.quark` for upstream work.
  - Launcher now auto-include-paths the vendored Quark when a user install is not present.

- 2026-01-04 — Architecture pivot: Runnables + HTTP instead of LSP Code Actions
  - Researched Zed extension API limitations (no `workspace/executeCommand` support, issue #13756).
  - Discovered Zed **runnables** system: Tree-sitter queries can tag code regions, matched tasks show play buttons in gutter.
  - Created `languages/SuperCollider/runnables.scm` to detect `(code_block)` and `(function_block)`.
  - **Validated:** Play buttons appear, `$ZED_CUSTOM_code` captures full block text including parentheses.
  - New architecture: Runnables -> Tasks -> HTTP POST -> sc_launcher -> LSP `supercollider.eval` -> sclang.

- 2026-01-04 — Added `supercollider.eval` command to LanguageServer.quark
  - Extended `ExecuteCommandProvider.sc` with direct eval command that takes raw source code.
  - No document URI required - enables HTTP-triggered evaluation.

- 2026-01-04 — Consolidated planning documents
  - Merged `CLAUDE_PLAN.md` findings into `PLAN.md` as the single source of truth.
  - Updated `AGENTS.md` to reflect dual-channel architecture (LSP for intelligence, HTTP for eval).
  - Deleted `CLAUDE_PLAN.md` (superseded).
  - M1 marked complete; M2 in progress (HTTP server implementation remaining).

- 2026-01-04 — Implemented HTTP eval server in sc_launcher
  - Added `tiny_http` dependency to launcher.
  - HTTP server listens on port 57130 (configurable via `--http-port`).
  - `POST /eval` sends `workspace/executeCommand` with `supercollider.eval` to sclang via UDP.
  - `GET /health` returns status check.
  - CORS headers included for potential browser-based tools.
  - Updated `.zed/tasks.json` with curl-based eval task.

- 2026-01-04 — Fixed ExecuteCommandProvider registration
  - Modified `LSP.sc` to register `ExecuteCommandProvider` at startup (not just after LSP init handshake).
  - This enables HTTP eval requests to work without requiring full LSP initialization.
  - **Validated end-to-end:** `curl -X POST -d "1 + 1" http://127.0.0.1:57130/eval` returns `> 2` in sclang output.
  - M2 complete.

- 2026-01-04 — Fixed task configuration for Zed compatibility
  - Removed trailing commas from `.zed/tasks.json` (JSON syntax error).
  - Wrapped curl command in `sh -c` to properly handle `$ZED_CUSTOM_code` variable expansion.
  - Changed from direct `curl` invocation to shell wrapper: `sh -c "curl -s -X POST --data-binary \"$ZED_CUSTOM_code\" ..."`
  - Updated PLAN.md with M3 status, current blockers, and next steps.

- 2026-01-04 — Identified launcher discovery issue
  - Play button executes task correctly, but HTTP server not running.
  - Root cause: Zed can't find `sc_launcher` binary (not in PATH, no settings configured).
  - Created `TESTING.md` with launcher configuration instructions and troubleshooting.
  - Task configuration working: `.zed/eval.sh` wrapper script with absolute path.

- 2026-01-04 — Play button workflow validated
  - Task execution confirmed working: play button → `.zed/eval.sh` → curl POST.
  - HTTP server starts successfully when launcher runs.
  - Verified full chain works when launcher is running in background.
  - **Key finding:** Launcher needs to be started by Zed's LSP system (not manually) for persistent operation.

- 2026-01-04 — Configured Zed settings for automatic launcher startup
  - Updated `~/.config/zed/settings.json` with launcher path and arguments.
  - Changed from debug to release build: `server/launcher/target/release/sc_launcher`.
  - Added `--http-port 57130` argument for HTTP eval server.
  - Created `test_eval.scd` with example code blocks.
  - Created `READY_TO_TEST.md` with complete testing instructions.
  - **Status:** Ready for end-to-end testing in Zed.

- 2026-01-04 — Session complete: M1-M3 done, ready for testing
  - Consolidated documentation to PLAN.md, AGENTS.md, LOG.md only.
  - Added quick start section to PLAN.md for next session.
  - Clarified build process: Extension (Zed rebuild) vs Launcher (cargo build).
  - Created `test_eval.scd` for evaluation testing.
  - **Status:** Architecture complete, configuration done, ready for end-to-end validation.

- 2026-01-04 — M3 validated: End-to-end evaluation working
  - Tested play button workflow in Zed: click play → task executes → HTTP POST → sclang evaluates → result printed.
  - Fixed CodeLensProvider file write errors (wrapped in try blocks).
  - Fixed ExecuteCommandProvider "returning *itself*" warning:
    - Root cause: `^` (non-local return) inside functions stored in dictionaries bypasses `valueArray` return value capture.
    - Solution: Rewrote `supercollider.eval` and `supercollider.evaluateSelection` without `^`, using if/else expressions instead.
  - Clean logs confirmed: `> 2` result, proper JSON response `{"result":"2"}`, no warnings.
  - Copied fixes to system quark at `~/Library/Application Support/SuperCollider/downloaded-quarks/LanguageServer/`.
  - **M3 complete:** Evaluation via play button works cleanly.

- 2026-01-04 — M4 complete: Server control endpoints
  - Added HTTP endpoints to launcher:
    - `POST /stop` → `supercollider.internal.cmdPeriod` (hard stop all synths)
    - `POST /boot` → `supercollider.internal.bootServer` (boot scsynth)
    - `POST /recompile` → `supercollider.internal.recompile` (recompile class library)
    - `POST /quit` → `supercollider.internal.quitServer` (quit scsynth)
  - Added Zed tasks in `.zed/tasks.json` for all endpoints.
  - Refactored launcher with `send_command()` helper function.
  - All endpoints tested and working.
  - **M4 complete.**

---

## Next Session Tasks

1. **M5:** Documentation & polish
   - Keybindings guide
   - Enhanced snippets
   - User README

## Build Process Reference

**Extension (Rust → Wasm):**
- Change code in `src/lib.rs`
- In Zed: Extensions → Rebuild (or `zed: reload extensions`)
- No cargo command needed

**Launcher (Rust → Native):**
- Change code in `server/launcher/src/main.rs`  
- Run: `cd server/launcher && cargo build --release`
- Output: `server/launcher/target/release/sc_launcher`

**Quark changes:**
- Edit files in `server/quark/LanguageServer.quark/`
- Copy to system quark: `cp ... ~/Library/Application Support/SuperCollider/downloaded-quarks/LanguageServer/...`
- Kill sclang: `pkill -9 sclang`
- Reopen `.scd` file to restart LSP
