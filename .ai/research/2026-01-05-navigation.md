# LSP Navigation Research

Date: 2026-01-05

## Problem

Go-to-definition and references weren't working. Zed LSP server was running but navigation requests were never sent.

## Root Cause

Two issues were blocking navigation:

1. **Server bugs:** Uninitialized arrays and race conditions in LanguageServer.quark caused crashes
2. **Config fields:** `opt_into_language_servers` and `scope_opt_in_language_servers` in config.toml prevented Zed from sending LSP requests

Diagnosis: Log analysis (`grep -i "definition" /tmp/sc_launcher_stdin.log`) showed zero requests from Zed, revealing a client-side configuration issue rather than server bug.

## Solution

### Server Fixes
Fixed in `server/quark/LanguageServer.quark/`:
- Initialize arrays before use (LSPDatabase.sc)
- Handle nil dictionary keys safely
- Handle didChange/didOpen race conditions (TextDocumentProvider.sc)

### Config Fix
Removed these fields from `languages/SuperCollider/config.toml`:
```toml
opt_into_language_servers = ["supercollider"]
scope_opt_in_language_servers = ["supercollider"]
```

These fields work for built-in Zed languages but break extension-provided languages. Minimal config with only documented fields resolved the issue.

## Key Learnings

### Zed Extension System
- Built-in and extension-provided languages behave differently
- Extension config should be minimal - only use documented fields from [Zed docs](https://zed.dev/docs/extensions/languages)
- Reference working extensions: [Erlang](https://github.com/zed-extensions/erlang), [Elixir](https://github.com/zed-extensions/elixir)

### Debugging Approach
- Log analysis revealed client vs server issue
- Comparative analysis with working extensions identified config difference
- Tree-sitter is for syntax highlighting; LSP handles navigation

## Follow-up Work

Additional LSP features implemented after navigation fix:
- Document rehydration for didOpen/didChange race conditions (2026-01-06)
- HoverProvider, outline support
- Additional capability advertisement

**LSP capability testing checklist:**
- textDocument/definition, textDocument/references (working)
- textDocument/hover, textDocument/completion
- textDocument/signatureHelp, textDocument/documentSymbol
- textDocument/codeLens, workspace/symbol
- Cross-file navigation, large file handling

See `.ai/improvements.md` for future enhancement priorities.

## Debugging Commands

```bash
# Monitor LSP requests
grep -i "definition\|references" /tmp/sc_launcher_stdin.log

# Check sclang errors
grep -i "error\|dnu" /tmp/sclang_post.log

# Force restart
pkill -9 sc_launcher; pkill -9 sclang
```

See `.ai/commands.md` for complete debugging reference.
