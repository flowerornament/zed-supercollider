# Zed SuperCollider Migration Plan (Navigator)

This plan ports core scnvim workflows to a first‑class Zed extension with clear milestones and acceptance criteria.

## 0) Migration Brief
- Goal: match day‑to‑day scnvim workflow
  - Evaluate line/selection/block in `sclang`
  - Show post window output
  - Start/stop/recompile `sclang`; boot/hard‑stop `scsynth`
  - Inline help lookup + docs
  - Completion, hover, go‑to‑definition
  - Snippets

### ADR‑001 (High‑Level Decisions)
1) Language intelligence via LSP, not custom pipes. Reuse LanguageServer.quark with a small launcher/bridge for stdio LSP.
2) Syntax & structure via Tree‑sitter. Package `tree-sitter-supercollider` and provide Zed queries.
3) Evaluation & post window
   - Primary: Zed runnables + Tasks → HTTP → launcher → sclang.
   - Post output surfaces in the terminal panel; optional persistent `sclang` task as fallback.
4) Zed extension in Rust→Wasm using `zed_extension_api` (process, lsp, settings).

## 1) Repository Layout
```
zed-supercollider/
  extension.toml
  Cargo.toml
  src/lib.rs
  languages/SuperCollider/
    config.toml
    highlights.scm
    brackets.scm
    indents.scm
    outline.scm
    injections.scm          # optional
    textobjects.scm         # optional (Vim-mode motions)
  snippets/
    supercollider.json
  server/
    launcher/
      Cargo.toml
      src/main.rs
    quark/                  # optional submodule/downloader
  docs/
    MIGRATION.md
    MIGRATION_PLAN.md
    SETTINGS.md
    TASKS_SNIPPET.md
    TROUBLESHOOTING.md
    USAGE.md
    KEYBINDINGS.md
  tests/
    fixtures/
```
- `extension.toml` declares metadata and pins the Tree‑sitter grammar repo.
- `src/lib.rs` registers the extension and LSP via `zed_extension_api`.

## 2) Language Support: Grammar + Queries
- Pin `tree-sitter-supercollider` in `extension.toml`:
```
[grammars.supercollider]
repository = "https://github.com/madskjeldgaard/tree-sitter-supercollider"
rev = "<pinned-commit-sha>"
```
- `languages/SuperCollider/config.toml`:
```
name = "SuperCollider"
grammar = "supercollider"
path_suffixes = ["sc", "scd"]
line_comments = ["// "]
tab_size = 4
hard_tabs = false
```
- Provide `highlights.scm`, `brackets.scm`, `indents.scm`, `outline.scm`.
- Runnable detection: outline/runnables for `{ ... }`, `( ... )`, and comment‑delimited regions.

## 3) LSP Server: Bootstrap & Integration
- Write `server/launcher` (Rust) to:
  - Detect/install LanguageServer.quark.
  - Start `sclang` loading the LSP server.
  - Bridge stdio JSON‑RPC to whatever transport the Quark expects.
- In `src/lib.rs`, register the LSP for SuperCollider using `zed_extension_api::lsp` and expose settings.
- Custom LSP commands (server-side):
  - `supercollider.eval` accepts raw source (used by the launcher's HTTP `/eval` endpoint).
  - Server control endpoints map to `CmdPeriod.run`, `thisProcess.recompile`, `s.boot`, `s.quit`.

## 4) Post Window Strategy
- Primary: task/launcher output in the terminal panel (runnables → tasks → HTTP eval).
- Fallback: document a user Task snippet to run a persistent `sclang` terminal in Zed’s Tasks panel.

## 5) User Commands & Keymaps
- Map tasks to scnvim semantics: eval block (runnables), boot/hard-stop, recompile, open help.
- Document keybindings in `README.md` (tasks are bound via user `keymap.json`).

## 6) Documentation & Help
- Preferred: ask LanguageServer.quark for docs (hover/requests returning Markdown) and open in a new buffer.
- Optional: external converter for `.schelp` → Markdown; open rendered buffer.

## 7) Snippets
- `snippets/supercollider.json` with SynthDef/UGen/Pattern templates (prefixes mirroring scnvim where sensible).

## 8) Settings & Environment
- Configure `sclang` path and logging via launcher arguments; LSP settings live under `lsp.supercollider.settings.supercollider`.
- Evaluation HTTP port is configurable in the launcher (default `57130`).

## 9) Tasks
- Eval tasks: runnables tagged `sc-eval` POST `$ZED_CUSTOM_code` to the launcher's `/eval`.
- Server control tasks: `/boot`, `/stop`, `/recompile`, `/quit`.
- Optional fallback: `SC: Start sclang (post)` in an integrated terminal.

## 10) Milestones (LLM‑Executable)
- M1 Language skeleton
  1. Scaffold extension; register `MyExtension`; CI for `wasm32-wasi`.
  2. Pin grammar; add `languages/SuperCollider/config.toml`.
  3. Add minimal queries; verify highlighting/brackets/outline.
  - Acceptance: SC files open with highlighting, brackets, outline.
- M2 LSP bootstrap
  4. Create `server/launcher` to start Quark LSP and bridge stdio.
  5. Register LSP in `src/lib.rs`.
  6. Implement settings + health check.
  7. Add HTTP eval server endpoints (`/eval`, `/stop`, `/boot`, `/quit`, `/recompile`).
  - Acceptance: hover, completion, go‑to‑definition, diagnostics.
- M3 Evaluate + Post
  8. Runnables tag eval blocks; tasks POST `$ZED_CUSTOM_code` to `/eval`.
  9. Document terminal panel workflow and fallback post task.
  - Acceptance: play button/task evals work; output visible; hard stop works.
- M4 Help & Docs
  10. `open help` via LSP; render Markdown buffer.
  11. Optional converter path for `.schelp`.
  - Acceptance: hover shows signatures; help buffer opens.
- M5 Snippets & Polish
  12. Add snippets.
  13. Default keybindings mirroring SCIDE/scnvim.
  14. Draft `MIGRATION.md` and `TROUBLESHOOTING.md`.

## 11) Feature Parity Mapping
- send_block → runnables + tasks + HTTP (line/selection optional if available)
- start/stop/eval/recompile/hard_stop → tasks to launcher HTTP endpoints
- post window → terminal panel output from launcher/tasks; fallback integrated terminal
- help → LSP hover/requests; optional converter
- snippets → static JSON
- syntax/indents/outline → Tree‑sitter + queries

## 12) Risks & Mitigations
- Transport mismatch (UDP vs stdio): launcher normalizes; long‑term Quark PR.
- Output visibility: terminal panel is primary; keep persistent `sclang` fallback.
- Conflicts with SCIDE Document: document in troubleshooting; allow dedicated `sclang_conf.yaml`.

## 13) Concrete Prompts (per Milestone)
- M1: scaffold extension and grammar pin.
- M2: implement launcher and register LSP.
- M3: add runnables + tasks eval and terminal fallback.
- M4: implement help request and optional converter.
- M5: snippets + docs drafts.

## 14) Acceptance Test Script (Manual)
1) Open an example project; evaluate a block with SinOsc (play button/task); hear audio and see post output.
2) Hover SinOsc; see signature/doc excerpt.
3) Trigger hard stop; audio stops; post logs.
4) Recompile class library; completions resume.

## 15) Notes for Life OS
- Pipeline: Systems/Ops → “Zed SuperCollider Extension” (Active)
- Now Horizon: M1→M3; Season Horizon: M4→M5.
- Character mix: Navigator (architecture), Pruner (scope), Musician (snippets/docs).

## Key References
- scnvim README and modules
- Zed extension model (Rust→Wasm), language queries, snippets, tasks/terminal
- Tree‑sitter SuperCollider grammar
- VS Code extension & LanguageServer.quark
- `sclang` CLI setup
