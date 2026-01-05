# SuperCollider Zed Extension

## What This Is
Zed extension for SuperCollider with LSP support and HTTP-based code evaluation.

**Architecture:** 3-layer bridge (Zed WASM → Rust Launcher → sclang UDP)
**Unique design:** Dual-channel (LSP for intelligence, HTTP for evaluation)
**Why HTTP:** Zed extension API cannot invoke workspace/executeCommand (see .ai/decisions/001-http-not-lsp.md)

## Current State

**Status (2026-01-07, latest):** Hover works with class doc block. Find References fallback fixed for symbols like `SinOscFB`; built-ins like `MouseX`/`.postln` still behave oddly and need follow-up. Outline still empty (Zed never sends `textDocument/documentSymbol`). Completion/eval/server-control OK. Current reference errors seen in logs were “Non Boolean in test” coming from the references provider; repo version has a fix, but the installed quark must match it.

**Working:** go-to-definition, hover, completion, eval, server control.
**Partial:** references (ok for SinOscFB; built-ins need doc/fallback tuning).
**Missing:** outline (no `textDocument/documentSymbol` from Zed); signature help unverified.
**See:** `.ai/improvements.md` for prioritized enhancement backlog.

## Quick File Map

| Path | What |
|------|------|
| `src/lib.rs` | Extension entry (WASM) |
| `server/launcher/src/main.rs` | LSP bridge + HTTP server |
| `server/quark/LanguageServer.quark/` | Vendored LSP implementation |
| `languages/SuperCollider/runnables.scm` | Play button detection |
| `languages/SuperCollider/config.toml` | **KEEP MINIMAL** (see anti-patterns) |
| `.zed/tasks.json` | Evaluation and control tasks |
| `.ai/improvements.md` | Prioritized enhancement backlog |
| `.ai/tasks/2026-01-07-lsp.md` | Latest LSP debugging notes and next steps |
| `.ai/research/2026-01-05-navigation.md` | Go-to-definition investigation and follow-ups |

## Build & Test

**Build extension:** In Zed: Extensions → Rebuild (or cmd-shift-p → "reload extensions")

**Build launcher:**
```bash
cd server/launcher && cargo build --release
```

**Verify LSP requests:**
```bash
grep -i "definition\|references" /tmp/sc_launcher_stdin.log
```

**Verify evaluation endpoint:**
```bash
curl -X POST -d "1 + 1" http://127.0.0.1:57130/eval
```

**Check for errors:**
```bash
grep -i "error\|exception\|dnu" /tmp/sclang_post.log
```

**Clean restart:**
```bash
pkill -9 sc_launcher; pkill -9 sclang
rm /tmp/sc_launcher_stdin.log /tmp/sclang_post.log
# reopen a .scd file in Zed after clearing logs

# If quark changes not picked up:
cp server/quark/LanguageServer.quark/Providers/HoverProvider.sc \
   ~/Library/Application\ Support/SuperCollider/downloaded-quarks/LanguageServer/Providers/HoverProvider.sc
pkill -9 sclang
```

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

## Essential Patterns

### Initialize SC classvars in *initClass

```supercollider
TextDocumentProvider {
    classvar <pendingOpens, <pendingChanges, <initialized;

    *initClass {
        pendingOpens = Array.new;
        pendingChanges = Array.new;
        initialized = false;
    }
}
```

**Why:** Prevents DNUs when provider code loads before initialization.

### Handle nil dictionary keys

```supercollider
allMethodsByName[method.name] = (allMethodsByName[method.name] ?? { Array.new }).add(method);
```

### Copy Quark changes to system

After editing files in `server/quark/LanguageServer.quark/`:
```bash
cp server/quark/LanguageServer.quark/File.sc \
   ~/Library/Application\ Support/SuperCollider/downloaded-quarks/LanguageServer/File.sc
pkill -9 sclang
```

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
- `.ai/improvements.md` - Enhancement backlog
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
