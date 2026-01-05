# SuperCollider Zed Extension

## Purpose
Ship a stable Zed extension for SuperCollider: navigation, hover/completion, and play-button evaluation via a dual-channel bridge (Zed WASM → Rust launcher → sclang over UDP/HTTP). HTTP is used for eval because Zed extensions cannot invoke `workspace/executeCommand` (see .ai/decisions/001-http-not-lsp.md).

## Quick Start for Agents
- Read `.ai/tasks/2026-01-05-execution-plan.md` (single source of truth for priorities).
- Honor anti-patterns below (config.toml minimal; no `^` returns in SC dicts; do not overwrite user-installed quark).
- Use vendored quark for changes; never revert user modifications.
- Task docs now use YAML front matter for status tracking (see `.ai/tasks/README.md`). Update `status`/`updated` and append to the `## Status Log` when making progress.
- README is user-facing; contributor docs live here in `.ai/` (this file is the entry point; see `.ai/README.md` for the map).

## Architecture (mental model)
- **Zed Extension (WASM)**: `src/lib.rs` selects launcher command, merges settings, and passes LSP traffic through stdio.
- **Launcher (Rust)**: `server/launcher/src/main.rs` translates stdio↔UDP for LSP, buffers until `***LSP READY***`, hosts HTTP server (`/eval`, `/stop`, `/boot`, `/recompile`, `/quit`), and spawns/manages `sclang --daemon`.
- **LanguageServer.quark (SuperCollider)**: Providers for definition/references/hover/completion/executeCommand, plus LSPDatabase indexing. Communicates over UDP.
- **Dual channels**: LSP over stdio↔UDP for intelligence; HTTP → UDP for evaluation/control (play buttons and tasks).

## Current State

**Status (2026-01-05):** Hover works with class doc block. References work for `SinOscFB`; built-ins (`MouseX`, `.postln`) still odd. Outline empty (Zed never sends `textDocument/documentSymbol`). Completion/eval/server-control OK. Vendored quark in use; “Non Boolean in test” still appears (references provider needs hardening).

**Working:** go-to-definition, hover, completion, eval, server control.

**Partial:** references (built-ins need fallback tuning).

**Missing:** outline (no `textDocument/documentSymbol` requests); signature help unverified.

**Priorities:** Follow P0 items in `.ai/tasks/2026-01-05-execution-plan.md` (remove serverStatus, safe dev launcher detection, safe shutdown, capability hygiene, probe JSON fix, tasks/config/logging/error-message hardening, quark safety, docs tokenization).

## Quick File Map

| Path | What |
|------|------|
| `src/lib.rs` | Extension entry (WASM) |
| `server/launcher/src/main.rs` | LSP bridge + HTTP server |
| `server/quark/LanguageServer.quark/` | Vendored LSP implementation |
| `languages/SuperCollider/runnables.scm` | Play button detection |
| `languages/SuperCollider/config.toml` | **KEEP MINIMAL** (see anti-patterns) |
| `.zed/tasks.json` | Evaluation and control tasks |
| `.ai/tasks/2026-01-05-execution-plan.md` | Consolidated backlog/plan (P0–P3) |
| `.ai/tasks/2026-01-05-lsp.md` | Latest LSP debugging notes and next steps |
| `.ai/research/2026-01-05-navigation.md` | Go-to-definition investigation and follow-ups |

## Build/Test (quick pointers)
- Extension rebuild: In Zed → Extensions → Rebuild (or cmd-shift-p → reload extensions).
- Launcher build: `cd server/launcher && cargo build --release`.
- Quick checks: `grep -i "definition\|references" /tmp/sc_launcher_stdin.log`; `curl -X POST -d "1 + 1" http://127.0.0.1:57130/eval`; `grep -i "error\\|exception\\|dnu" /tmp/sclang_post.log`.
- Full command references in `.ai/commands.md`.

## Critical Anti-Patterns

### DO NOT add these fields to config.toml

```toml
opt_into_language_servers = ["supercollider"]
scope_opt_in_language_servers = ["supercollider"]
```

**Why:** These fields work for built-in Zed languages but break extension-provided languages. They prevented Zed from sending LSP definition requests.

**Correct minimal config.toml:**
```toml
name = "SuperCollider"
grammar = "supercollider"
path_suffixes = ["sc", "scd"]
line_comments = ["// "]
tab_size = 4
hard_tabs = false
```

Only add documented fields from https://zed.dev/docs/extensions/languages

### DO NOT use ^ (non-local return) in SC dictionaries

```supercollider
// BAD - returns provider itself
commands = (
    'supercollider.eval': { |params|
        ^("result": params["source"].interpret)
    }
);

// GOOD - returns result
commands = (
    'supercollider.eval': { |params|
        ("result": params["source"].interpret)
    }
);
```

**Why:** `^` bypasses `valueArray` return capture, returns provider object instead of result.

### DO NOT assume LSP executeCommand available

Zed extension API cannot invoke `workspace/executeCommand` (Issue #13756). Use HTTP channel instead (see .ai/decisions/001-http-not-lsp.md).

## Prep Checklist (before coding)
- Ensure dev launcher built only when present; otherwise rely on PATH/settings.
- Keep language config minimal; run `scripts/validate-config.sh` if present.
- Use vendored quark for edits; do not overwrite user-installed quark without consent.
- Clean logs if testing locally: remove `/tmp/sc_launcher_stdin.log` and `/tmp/sclang_post.log`; restart launcher/sclang as needed.
- Keep docs in sync: when state changes, update `.ai/tasks/2026-01-05-execution-plan.md`, this context, and add research notes as needed; follow existing doc structure.
- Git hygiene: make small, focused commits (code + related doc updates together), avoid force pushes/reverts of user changes, and mirror the existing concise style (e.g., `fix(extension): ...`, `docs: ...`). Commit when you land a coherent step; keep working tree clean between steps. Include submodule bumps when you change vendored quark files: commit inside `server/quark/LanguageServer.quark` first, then update the parent repo to point to the new submodule SHA.

## Essential Patterns (SuperCollider)
- Initialize classvars in `*initClass`.
- Handle nil dictionary keys: `(dict[key] ?? { Array.new }).add(...)`.
- Avoid `^` in dictionary functions; use expression returns.
- Copy vendored quark to system only when intentionally testing there; prefer vendored path during development.

## Known Limitations (Don't Fix These)

These are expected behavior, not bugs:

- **Hover docs:** Not implemented in LanguageServer.quark (Quark limitation, not extension)
- **Terminal flash:** Zed creates/destroys terminals for tasks (Zed limitation, issue tracked)
- **Post window duplicates:** Zed tasks don't support singleton/toggle behavior
- **Inline diagnostics:** Not in LanguageServer.quark yet

If user reports these as "not working", explain they're known limitations.

## Required User Setup

Context for debugging "user says it doesn't work" issues:

1. **LanguageServer.quark installed:**
   ```supercollider
   Quarks.install("LanguageServer");
   ```

2. **Launcher path configured** in `~/.config/zed/settings.json`:
   ```json
   {
     "lsp": {
       "supercollider": {
         "binary": {
           "path": "/path/to/sc_launcher",
           "arguments": ["--mode", "lsp", "--http-port", "57130"]
         }
       }
     }
   }
   ```

3. **Tasks created** in `.zed/tasks.json` (see .ai/commands.md)

## Common Tasks

**Debug LSP issue:** See `.ai/prompts/debug-lsp-issue.md`

**Add HTTP endpoint:**
1. Add handler in `server/launcher/src/main.rs` HTTP server section
2. Add SC command in `ExecuteCommandProvider.sc`
3. Add task in `.zed/tasks.json`

**Add LSP capability:**
1. Implement provider in Quark
2. Register in `LSP.sc`
3. Advertise in launcher `initialize` response
4. No extension code change needed (passes through)

## Verification After Changes

**After Quark changes:**
```bash
pkill -9 sclang
grep -i "error\|dnu" /tmp/sclang_post.log
```

**After config changes:**
```bash
grep -i "definition" /tmp/sc_launcher_stdin.log
```

**After launcher changes:**
```bash
curl -X POST -d "1 + 1" http://127.0.0.1:57130/eval
```

## Key Implementation Notes

**Doc sync:** Providers rehydrate from `TextDocumentProvider.lastOpenByUri` cache when doc isn't open, handling didOpen/didChange race conditions.

**Logging:** Use `info` level during debugging/verification, reduce to `warning` once features are stable. Key logs in `/tmp/sclang_post.log` and `/tmp/sc_launcher_stdin.log`.

**Git hygiene:** Avoid destructive commands, keep commits focused, never revert user changes.

**Verification checklist:** When testing LSP features, verify hover, references, outline, code lens, signature help, workspace symbols, cross-file navigation.

## Documentation

- `.ai/architecture.md` - System design and data flows
- `.ai/conventions.md` - Code patterns and anti-patterns
- `.ai/commands.md` - Build/test/debug commands
- `.ai/tasks/2026-01-05-execution-plan.md` - Consolidated enhancement/backlog plan
- `.ai/decisions/` - Architectural Decision Records
- `.ai/research/` - Investigation findings
- `.ai/prompts/` - Task templates

## Coding Conventions

**Rust:** 4-space indent, `snake_case` modules, `PascalCase` types
**SuperCollider:** Initialize arrays, handle nil keys, no `^` in dictionary functions
**Tree-sitter:** Small composable queries, precise captures

## Resources

- [Zed Language Extensions](https://zed.dev/docs/extensions/languages)
- [LSP Specification](https://microsoft.github.io/language-server-protocol/)
- [LanguageServer.quark](https://github.com/scztt/LanguageServer.quark)
- Zed Issue #13756: workspace/executeCommand limitation
