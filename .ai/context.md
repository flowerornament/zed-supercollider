# SuperCollider Zed Extension

## What This Is
Zed extension for SuperCollider with LSP support and HTTP-based code evaluation.

**Architecture:** 3-layer bridge (Zed WASM → Rust Launcher → sclang UDP)
**Unique design:** Dual-channel (LSP for intelligence, HTTP for evaluation)
**Why HTTP:** Zed extension API cannot invoke workspace/executeCommand (see .ai/decisions/001-http-not-lsp.md)

## Current State

**Status:** Core features working; navigation now functional after doc rehydration fixes
**Last change:** 2026-01-06 - Added doc hydration/cache for didOpen, fixed pending queue, added HoverProvider
**Next task:** Verify hover/outline/references in Zed with rehydration (log level currently `info`)

**What works:**
- ✅ Go-to-definition and find references (with doc rehydrate fallback)
- ✅ Code evaluation via play buttons (HTTP to :57130)
- ✅ Server control (boot/stop/recompile)
- ✅ Syntax highlighting and completions
- ✅ Hover provider implemented (returns symbol markdown), outline rehydrates doc

**What's broken/pending:**
- Hover/outline/ref in-editor re-verify after latest changes
- Custom notification warning (non-blocking)

**What needs attention:**
- Full LSP capability verification (see .ai/research/2026-01-05-navigation.md)
- Reduce log level after verification (currently `info`)

**Improvement backlog:**
- See IMPROVEMENTS.md for 28+ prioritized enhancements
- High priority: custom notification fix, performance optimization, testing infrastructure

## Quick File Map

| Path | What |
|------|------|
| `src/lib.rs` | Extension entry (WASM) |
| `server/launcher/src/main.rs` | LSP bridge + HTTP server |
| `server/quark/LanguageServer.quark/` | Vendored LSP implementation |
| `languages/SuperCollider/runnables.scm` | Play button detection |
| `languages/SuperCollider/config.toml` | **KEEP MINIMAL** (see anti-patterns) |
| `.zed/tasks.json` | Evaluation and control tasks |
| `IMPROVEMENTS.md` | Prioritized enhancement backlog |
| `.ai/research/2026-01-05-navigation.md` | Go-to-definition investigation and follow-ups |

## Build & Test

**Build extension:** In Zed: Extensions → Rebuild (or `cmd-shift-p` → "reload extensions")
**Build launcher:** `cd server/launcher && cargo build --release`

**Verify navigation:**
```bash
grep -i "definition\|references" /tmp/sc_launcher_stdin.log
# Should show multiple requests
```

**Verify evaluation:**
```bash
curl -X POST -d "1 + 1" http://127.0.0.1:57130/eval
# Should return: {"result":"2"}
```

**Check for errors:**
```bash
grep -i "error\|exception\|dnu" /tmp/sclang_post.log
```

## Critical Anti-Patterns

### ❌ DO NOT add these fields to config.toml
```toml
opt_into_language_servers = ["supercollider"]
scope_opt_in_language_servers = ["supercollider"]
```
**Evidence:** These exact fields prevented Zed from sending definition requests (2026-01-05).
Verified by `grep -i "definition" /tmp/sc_launcher_stdin.log` showing zero requests with fields,
multiple requests after removal.

**Why they break:** Work for built-in Zed languages (TSX, Rust), break extension-provided languages.

**Correct config.toml:**
```toml
name = "SuperCollider"
grammar = "supercollider"
path_suffixes = ["sc", "scd"]
line_comments = ["// "]
tab_size = 4
hard_tabs = false
```
Only add documented fields from https://zed.dev/docs/extensions/languages

### ❌ DO NOT use ^ (non-local return) in SC dictionaries
```supercollider
// BAD
commands = (
    'supercollider.eval': { |params|
        ^("result": params["source"].interpret)  // ❌ Returns provider itself
    }
);

// GOOD
commands = (
    'supercollider.eval': { |params|
        ("result": params["source"].interpret)  // ✅ Returns result
    }
);
```
**Evidence:** Caused "returning *itself*" warnings in ExecuteCommandProvider (2026-01-04).
**Why:** `^` bypasses `valueArray` return capture, returns provider object instead of result.

### ❌ DO NOT assume LSP executeCommand available
Zed extension API cannot invoke `workspace/executeCommand` (Issue #13756).
Use HTTP channel instead (see .ai/decisions/001-http-not-lsp.md).

## Essential Patterns

### ✅ Initialize SC classvars in *initClass
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

### ✅ Handle nil dictionary keys
```supercollider
allMethodsByName[method.name] = (allMethodsByName[method.name] ?? { Array.new }).add(method);
```

### ✅ Copy Quark changes to system
After editing files in `server/quark/LanguageServer.quark/`:
```bash
cp server/quark/LanguageServer.quark/File.sc \
   ~/Library/Application\ Support/SuperCollider/downloaded-quarks/LanguageServer/File.sc
pkill -9 sclang  # Force reload
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

## Operational Notes

- Doc sync strategy: Providers rehydrate from `TextDocumentProvider.lastOpenByUri` when a doc isn’t open/has no string. This is intentional to survive early didOpen/didChange ordering.
- Logging posture: Currently at `info` for hydration verification; lower to `warning` once hover/outline/references are confirmed stable.
- Post-fix verification checklist: hover, references, outline (documentSymbol), code lens, signature help, workspace symbols, cross-file navigation.

## Recent LSP fixes (2026-01-06)
- Pending `didOpen`/`didChange` queued and replayed; equality now string-based to avoid misclassification.
- Cached last `didOpen` per URI; providers (definition/references/documentSymbol/hover) rehydrate docs when not open.
- Added HoverProvider (basic markdown echo) and document-symbol rehydrate; default log level set to `info` temporarily for debugging.

**After config changes:**
```bash
grep -i "definition" /tmp/sc_launcher_stdin.log
```

**After launcher changes:**
```bash
curl -X POST -d "1 + 1" http://127.0.0.1:57130/eval
```

**Clean restart:**
```bash
pkill -9 sc_launcher; pkill -9 sclang
rm /tmp/sc_launcher_stdin.log /tmp/sclang_post.log
# Reopen .scd file in Zed
```

## Documentation

**Architecture deep dive:** `.ai/architecture.md`
**Code patterns:** `.ai/conventions.md`
**Command reference:** `.ai/commands.md`
**Decision history:** `.ai/decisions/`
**Research notes:** `.ai/research/`
**Task templates:** `.ai/prompts/`
**Improvements:** `IMPROVEMENTS.md`

## Coding Conventions

**Rust:** 4-space indent, ~100 char lines, `snake_case` modules, `PascalCase` types
**SuperCollider:** Initialize arrays, handle nil keys, no `^` in dictionary functions
**Tree-sitter:** Small composable queries, precise captures

## Key Resources

- [Zed Language Extensions](https://zed.dev/docs/extensions/languages)
- [LSP Specification](https://microsoft.github.io/language-server-protocol/)
- [LanguageServer.quark](https://github.com/scztt/LanguageServer.quark)
- Zed Issue #13756: workspace/executeCommand limitation
