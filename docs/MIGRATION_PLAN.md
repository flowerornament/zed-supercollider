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
   - Primary: LSP custom commands streaming output via `window/logMessage`.
   - Fallback: Zed Tasks + Integrated Terminal running persistent `sclang`.
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
  tasks/
    sc-post.ztask.json
  docs/
    README.md
    MIGRATION.md
    TROUBLESHOOTING.md
    MIGRATION_PLAN.md
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
- Custom LSP commands:
  - `supercollider.evalBlock|evalLine|evalSelection|hardStop|recompile|bootServer|quitServer`.
  - Semantics: `hardStop` → `CmdPeriod.run`; `recompile` → `thisProcess.recompile`; `bootServer` → `s.boot`; `quitServer` → `s.quit`.

## 4) Post Window Strategy
- Primary: stream via `window/logMessage` into Zed’s output panel.
- Fallback: document a user Task snippet to run a persistent `sclang` terminal in Zed’s Tasks panel; optionally provide an extension command to spawn `sclang` in a terminal if supported by the API.

## 5) User Commands & Keymaps
- Map Zed actions to scnvim semantics: eval selection/line/block; boot/hard‑stop; recompile; open help.
- Default keybindings in `extension.toml`; document overrides in `README.md`.

## 6) Documentation & Help
- Preferred: ask LanguageServer.quark for docs (hover/requests returning Markdown) and open in a new buffer.
- Optional: external converter for `.schelp` → Markdown; open rendered buffer.

## 7) Snippets
- `snippets/supercollider.json` with SynthDef/UGen/Pattern templates (prefixes mirroring scnvim where sensible).

## 8) Settings & Environment
- Settings: `supercollider.sclangPath`, `supercollider.confYamlPath?`, `supercollider.postMode`, `supercollider.autoBootServer`, `supercollider.help.converter`.
- “Check setup” command probes: `sclang -h`, simple eval `1 + 1`, optional boot/quit.

## 9) Fallback Tasks
- `SC: Start sclang (post)` → integrated terminal `sclang`.
- Optional: action to send selection to terminal when LSP unavailable.

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
  - Acceptance: hover, completion, go‑to‑definition, diagnostics.
- M3 Evaluate + Post
  7. Client eval commands compute regions via Tree‑sitter and `executeCommand`.
  8. Server evaluates and forwards output as `logMessage`.
  9. Fallback Task for terminal post window.
  - Acceptance: key evals work; output visible; hard stop works.
- M4 Help & Docs
  10. `open help` via LSP; render Markdown buffer.
  11. Optional converter path for `.schelp`.
  - Acceptance: hover shows signatures; help buffer opens.
- M5 Snippets & Polish
  12. Add snippets.
  13. Default keybindings mirroring SCIDE/scnvim.
  14. Draft `MIGRATION.md` and `TROUBLESHOOTING.md`.

## 11) Feature Parity Mapping
- send_line/block/selection → LSP `executeCommand` + Tree‑sitter regions
- start/stop/eval/recompile/hard_stop → LSP custom commands (+ terminal fallback)
- post window → LSP `logMessage`; fallback integrated terminal
- help → LSP hover/requests; optional converter
- snippets → static JSON
- syntax/indents/outline → Tree‑sitter + queries

## 12) Risks & Mitigations
- Transport mismatch (UDP vs stdio): launcher normalizes; long‑term Quark PR.
- Streaming output: prefer LSP messages; keep terminal fallback.
- Conflicts with SCIDE Document: document in troubleshooting; allow dedicated `sclang_conf.yaml`.

## 13) Concrete Prompts (per Milestone)
- M1: scaffold extension and grammar pin.
- M2: implement launcher and register LSP.
- M3: add eval commands and terminal task.
- M4: implement help request and optional converter.
- M5: snippets + docs drafts.

## 14) Acceptance Test Script (Manual)
1) Open an example project; evaluate selection with SinOsc; hear audio and see post output.
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
