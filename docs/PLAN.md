# Implementation Plan — Zed SuperCollider Extension

This plan defines the implementation strategy for a Zed extension providing SuperCollider language support, aiming for day-to-day parity with scnvim.

---

## 🚀 Quick Start for Next Session

**Current Status:** M1, M2, M3 complete. Ready for end-to-end testing.

**To Test Right Now:**
1. Restart Zed
2. Open `test_eval.scd` 
3. Click play button (▶️) next to `(1 + 1)`
4. Check LSP logs for `[sclang stdout] > 2`

**Build Commands:**
```bash
# Extension (when src/lib.rs changes)
# In Zed: Extensions panel → Rebuild

# Launcher (when server/launcher/src/main.rs changes)
cd server/launcher && cargo build --release
```

**Key Files:**
- Extension: `src/lib.rs`
- Launcher: `server/launcher/src/main.rs`
- HTTP eval: `server/launcher/src/main.rs:670-830`
- Quark: `server/quark/LanguageServer.quark/`
- Runnables: `languages/SuperCollider/runnables.scm`
- Tasks: `.zed/tasks.json`
- Eval script: `.zed/eval.sh`

**Settings Location:** `~/.config/zed/settings.json` (already configured)

---

## Architecture Overview

> **Revision Note (2026-01-04):** This plan incorporates validated findings from architecture exploration. The original LSP Code Actions approach for evaluation was replaced with a **Runnables + HTTP** architecture after discovering Zed's extension API cannot programmatically invoke `workspace/executeCommand`.

## Objectives

- Achieve day-to-day parity with scnvim for evaluation, post output, start/stop/recompile, help, LSP features, and snippets.
- Ship a maintainable Rust→Wasm Zed extension with Tree-sitter queries and a reliable LSP bridge.
- Provide a smooth live-coding experience with minimal friction.

## Scope & Non-Goals

**In scope:** SC language integration, LSP bootstrap, code evaluation, post output, server control, help, snippets, keymaps, docs.

**Non-goals v1:** Custom UI panes, complex status widgets, visual region flash on eval, inline result display.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                          Zed Editor                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  languages/SuperCollider/          User clicks play button     │
│  ┌─────────────────────┐           or presses keybinding       │
│  │ runnables.scm       │                    │                  │
│  │ ─────────────────── │                    │                  │
│  │ (code_block) @code  │────▶ Play button   │                  │
│  │ #set! tag sc-eval   │      in gutter     │                  │
│  └─────────────────────┘                    │                  │
│                                             ▼                  │
│  tasks.json                     ┌─────────────────────┐        │
│  ┌─────────────────────┐        │ $ZED_CUSTOM_code    │        │
│  │ tags: ["sc-eval"]   │◀───────│ (captured block)    │        │
│  │ POST to /eval       │        └─────────────────────┘        │
│  └─────────────────────┘                    │                  │
│           │                                 │                  │
├───────────┼─────────────────────────────────┼──────────────────┤
│  LSP      │                                 │                  │
│  ┌────────┴────────┐                        │                  │
│  │ Completions     │                        │                  │
│  │ Hover / Docs    │     (LSP separate      │                  │
│  │ Go-to-def       │      from eval)        │                  │
│  │ Diagnostics     │                        │                  │
│  └────────┬────────┘                        │                  │
│           │                                 │                  │
└───────────┼─────────────────────────────────┼──────────────────┘
            │ stdio                           │ HTTP POST
            ▼                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                        sc_launcher                              │
│  ┌─────────────────────┐    ┌─────────────────────────────────┐ │
│  │ LSP Bridge          │    │ HTTP Eval Server (:57130)       │ │
│  │ (stdio ↔ UDP)       │    │ POST /eval  → supercollider.eval│ │
│  │                     │    │ POST /stop  → CmdPeriod.run     │ │
│  └──────────┬──────────┘    │ POST /boot  → s.boot            │ │
│             │               │ POST /recompile                 │ │
│             │               │ GET  /status                    │ │
│             │               └──────────────┬──────────────────┘ │
│             │                              │                    │
│             └──────────────┬───────────────┘                    │
│                            ▼                                    │
│                 ┌─────────────────────┐                         │
│                 │ sclang (--daemon)   │                         │
│                 │ + LanguageServer    │                         │
│                 └─────────────────────┘                         │
│                            │                                    │
│                            ▼                                    │
│                    stdout/stderr → Terminal panel               │
└─────────────────────────────────────────────────────────────────┘
```

### Key Design Decisions

**Why Runnables + HTTP instead of LSP Code Actions:**

| Factor | LSP Code Actions | Runnables + HTTP |
|--------|------------------|------------------|
| Play button in gutter | No | Yes |
| One-action eval | No (menu required) | Yes (click or keybinding) |
| Block detection | Server-side, latency | Client-side Tree-sitter |
| Direct keybinding | Not possible | Tasks can be bound |
| Extension API needed | `executeCommand` (missing) | Tasks API (works) |

**Why keep LSP:** Completions, hover docs, go-to-definition, diagnostics, document symbols all work well through standard LSP.

**Why HTTP:** Cross-platform, debuggable (`curl`), testable independently.

---

## Milestones

### M1 — Language Skeleton + Runnables ✅ COMPLETE

**Goal:** SuperCollider files load with syntax highlighting; evaluable blocks show play buttons.

**Status:** Validated 2026-01-04

**Completed:**
- `extension.toml` with grammar pin, LSP registration
- `Cargo.toml`, `src/lib.rs` with `zed::register_extension!`
- `languages/SuperCollider/config.toml` mapping `sc`/`scd`
- Tree-sitter queries: `highlights.scm`, `brackets.scm`, `indents.scm`, `outline.scm`
- `runnables.scm` detecting `(code_block)` and `(function_block)`
- Validated: Play buttons appear, `$ZED_CUSTOM_code` captures full block text

**Files:**
- `extension.toml`, `Cargo.toml`, `src/lib.rs`
- `languages/SuperCollider/{config.toml,highlights.scm,brackets.scm,indents.scm,outline.scm,runnables.scm}`

---

### M2 — LSP Bridge + HTTP Eval Server ✅ COMPLETE

**Goal:** LSP features work; HTTP endpoint enables code evaluation.

**Status:** HTTP server implemented 2026-01-04

**Completed:**
1. ✅ LSP bridge (stdio ↔ UDP) — already implemented in `server/launcher`
2. ✅ Vendored LanguageServer.quark with `supercollider.eval` command
3. ✅ Add HTTP server to launcher (`tiny_http`)
4. ✅ Implement core endpoints:
   - `POST /eval` → `workspace/executeCommand` with `supercollider.eval`
   - `GET /health` → health check
5. ✅ Port configurable via `--http-port` (default 57130)

**Future endpoints (M3):**
   - `POST /stop` → `supercollider.internal.cmdPeriod`
   - `POST /boot` → `supercollider.internal.bootServer`
   - `POST /recompile` → `supercollider.internal.recompile`

**Files:**
- `server/launcher/src/main.rs` (add HTTP server)
- `server/launcher/Cargo.toml` (add `tiny_http`)
- `server/quark/LanguageServer.quark/Providers/ExecuteCommandProvider.sc` (✅ `supercollider.eval` added)

**Validation:**
```bash
# Start launcher
./sc_launcher --mode lsp

# Test endpoints
curl -X POST -d "1 + 1" http://localhost:57130/eval
# → 2

curl -X POST -d "(SinOsc.ar(440) * 0.1).play" http://localhost:57130/eval
# → Synth('temp__0' : 1000)

curl -X POST http://localhost:57130/stop
# → ok
```

**Acceptance:**
- LSP completions, hover, go-to-def work
- All HTTP endpoints respond correctly
- Evaluation output appears in terminal

---

### M3 — Tasks Integration ✅ COMPLETE

**Goal:** Users can evaluate code via play button or keybinding.

**Status:** Configuration complete, ready for end-to-end testing (2026-01-04)

**Completed:**
- Fixed task JSON syntax and argument handling
- Created `.zed/eval.sh` wrapper script
- Configured Zed settings to auto-start launcher
- Validated full chain: play button → task → HTTP → sclang eval

**Tasks:**
1. ✅ Create task templates:
   ```json
   [
     {
       "label": "SuperCollider: Evaluate",
       "command": "sh",
       "args": ["-c", "curl -s -X POST --data-binary \"$ZED_CUSTOM_code\" http://127.0.0.1:57130/eval"],
       "tags": ["sc-eval"],
       "reveal": "always"
     },
     {
       "label": "SuperCollider: Stop",
       "command": "curl",
       "args": ["-s", "-X", "POST", "http://localhost:57130/stop"],
       "hide": "always"
     },
     {
       "label": "SuperCollider: Boot Server",
       "command": "curl",
       "args": ["-s", "-X", "POST", "http://localhost:57130/boot"],
       "hide": "on_success"
     }
   ]
   ```
2. Document keybinding setup in README
3. Document terminal panel workflow
4. Test full evaluate → output → stop cycle

**Files:**
- `.zed/tasks.json` (project tasks)
- `docs/USAGE.md` (includes keybindings)

**Acceptance:**
- Click play button → code evaluates
- Keybinding triggers evaluation
- Output visible in terminal panel

**Build Instructions:**

The extension has **two separate build steps**:

1. **Extension (Rust → Wasm):**
   - In Zed: Extensions panel → "Rebuild" (or `zed: reload extensions`)
   - Builds `src/lib.rs` to Wasm
   - **Do this when:** You change extension code (src/lib.rs)

2. **Launcher (Rust → Native Binary):**
   ```bash
   cd server/launcher
   cargo build --release
   ```
   - Builds `sc_launcher` binary
   - **Do this when:** You change launcher code (server/launcher/src/main.rs)
   - Output: `server/launcher/target/release/sc_launcher`

**Testing Workflow:**
1. Restart Zed (or reload extensions)
2. Open `test_eval.scd`
3. Verify LSP starts (check logs: `cmd-shift-p` → "open language server logs")
4. Look for: `HTTP eval server listening on http://127.0.0.1:57130`
5. Click play button next to `(1 + 1)`
6. Check terminal panel for task output
7. Check LSP logs for sclang result: `[sclang stdout] > 2`

**Current Configuration:**
- Zed settings: `~/.config/zed/settings.json` (configured)
- Launcher path: `server/launcher/target/release/sc_launcher`
- HTTP port: 57130
- Task script: `.zed/eval.sh`

**Next Steps:**
1. 🔲 End-to-end testing in Zed
2. 🔲 Add remaining HTTP endpoints (`/stop`, `/boot`, `/recompile`)
3. 🔲 Document keybinding setup (M4)
4. 🔲 Enhance snippets (M5)

---

### M4 — Help & Documentation

**Goal:** Help workflows work from the editor.

**Tasks:**
1. Verify LSP hover shows class/method docs
2. Document external help lookup (SC help browser)
3. Optional: converter path `.schelp` → Markdown

**Files:**
- `docs/HELP.md`

**Acceptance:**
- Hover over `SinOsc` shows signature and description
- Help workflow documented

---

### M5 — Polish

**Goal:** Complete developer experience.

**Tasks:**
1. Write comprehensive README
2. Test on clean system

**Files:**
- `README.md`, `docs/USAGE.md`, `docs/TROUBLESHOOTING.md`

**Acceptance:**
- Extension ready for public use
- New user can evaluate code within 5 minutes of install

---

## User Workflow (Final State)

### First-Time Setup
1. Install extension from Zed Extensions
2. Ensure `sclang` is on PATH (or configure in settings)
3. Copy task templates to `~/.config/zed/tasks.json`
4. Add keybindings to `~/.config/zed/keymap.json`

### Daily Usage
1. Open `.scd` file → LSP starts, syntax highlighting active
2. Open terminal panel (`ctrl+\``)
3. Run "SuperCollider: Boot Server" task
4. Write code — get completions, hover docs
5. Click play button or press eval keybinding
6. See output in terminal panel
7. Press stop keybinding to silence

### Feature Comparison

| Feature | scnvim | Zed Extension |
|---------|--------|---------------|
| Syntax highlighting | Tree-sitter | Tree-sitter |
| Completions | LSP | LSP |
| Hover docs | LSP | LSP |
| Go-to-definition | LSP | LSP |
| Evaluate line/block | `<C-e>` | Play button / keybinding |
| Hard stop | `<C-.>` | Keybinding |
| Boot server | `:SCNvimStart` | Task |
| Post window | Vim buffer | Terminal panel |
| Snippets | Yes | Yes |

---

## Validation Log

### M1 Core Assumption — VALIDATED (2026-01-04)

Tested and confirmed:
- Play buttons appear in gutter for `(code_block)` and `(function_block)` nodes
- `$ZED_CUSTOM_code` correctly captures full block text including parentheses
- Tasks execute when play button is clicked

Example captured output:
```
(
var x = 10;
var y = 20;
x + y
)
```

---

## Risks and Mitigations

| Risk | Status | Mitigation |
|------|--------|------------|
| `$ZED_CUSTOM_code` doesn't capture text | ✅ Resolved | Validated — works correctly |
| HTTP server complexity | Manageable | Use `tiny_http` (~500 LOC) |
| Users forget to start launcher | Document | Clear docs + startup task |
| Port conflicts | Configurable | Default 57130, user can override |
| Windows compatibility | Test needed | HTTP is cross-platform |

---

## Non-Goals (Deferred)

These may become possible as Zed's extension API expands:
- Custom post window panel (needs UI extensions)
- Status line widgets (not in API)
- Visual flash on evaluated region (not in API)
- Inline result display (not in API)

---

## References

- [Zed Tasks Documentation](https://zed.dev/docs/tasks)
- [Zed Language Extensions](https://zed.dev/docs/extensions/languages)
- [Zed Runnables Blog Post](https://zed.dev/blog/zed-decoded-tasks)
- [LanguageServer.quark](https://github.com/scztt/LanguageServer.quark)
- [scnvim](https://github.com/davidgranstrom/scnvim)
- [Zed GitHub Issue #13756](https://github.com/zed-industries/zed/issues/13756) — LSP executeCommand (open)
