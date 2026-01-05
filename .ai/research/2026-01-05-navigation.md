# LSP Navigation Research

## Date: 2026-01-05

## Overall Problem

Go-to-definition and other LSP navigation features weren't working in the zed-supercollider extension. When users tried to:
- Cmd+click on symbols
- Use Command Palette "Go to Definition"
- Navigate to references

Nothing happened. The LSP server was running, but Zed wasn't sending navigation requests.

### Initial Hypotheses
1. Server-side issues (capability advertisement, response format)
2. Tree-sitter configuration (missing textobjects.scm)
3. Extension implementation bugs
4. Zed client-side configuration issues

## How Solutions Were Found

### Phase 1: Server-Side Investigation
**Approach:** Examined server logs and code for errors

**Findings:**
- Fixed several bugs in SuperCollider Quark code:
  - `LSPDatabase.sc:203` - Uninitialized array causing crashes
  - `LSPDatabase.sc:222` - Nil dictionary key handling
  - `TextDocumentProvider.sc:115-118` - Race condition on didChange
- Server now starts cleanly and advertises capabilities correctly
- `/tmp/sclang_post.log` shows providers registered successfully

**Result:** Server issues fixed, but navigation still didn't work

### Phase 2: Log Analysis
**Approach:** Monitor LSP message flow between Zed and server

**Key Discovery:**
```bash
# Check what requests Zed is sending
grep -i "definition" /tmp/sc_launcher_stdin.log
# Result: EMPTY - no definition requests being sent
```

**Insight:** This is a client-side issue, not server-side. Zed isn't sending requests to the server.

### Phase 3: Comparative Analysis
**Approach:** Compare with working extensions

**Observations:**
- Rust extension: Shows status bar activity, go-to-definition works
- SuperCollider extension: No status bar activity, no definition requests
- Checked Erlang extension config (known working): No `opt_into_language_servers` field
- SuperCollider config HAD this field

**Hypothesis:** Config fields preventing Zed from enabling LSP features

### Phase 4: Configuration Fix
**Action:** Removed these fields from `languages/SuperCollider/config.toml`:
```diff
- opt_into_language_servers = ["supercollider"]
- scope_opt_in_language_servers = ["supercollider"]
```

**Verification:**
```bash
grep -i "definition" /tmp/sc_launcher_stdin.log
# Now shows multiple definition and references requests!
```

**Result:** ✅ Navigation working

## Solutions Implemented

### 1. Server Stability Fixes
**Files:**
- `server/quark/LanguageServer.quark/LSPDatabase.sc`
- `server/quark/LanguageServer.quark/Providers/TextDocumentProvider.sc`

**Changes:**
- Initialize arrays before use
- Handle nil dictionary keys safely
- Handle didChange arriving before didOpen (force document open)

### 2. Extension Configuration Fix
**File:** `languages/SuperCollider/config.toml`

**Problem:** Undocumented config fields that work for built-in Zed languages break extension-provided languages

**Solution:** Remove `opt_into_language_servers` and `scope_opt_in_language_servers`

**Correct minimal config:**
```toml
name = "SuperCollider"
grammar = "supercollider"
path_suffixes = ["sc", "scd"]
line_comments = ["// "]
tab_size = 4
hard_tabs = false
```

## Research Insights

### Zed Extension System
- **Built-in vs Extension languages behave differently** - config fields that work for TSX/Rust may break extensions
- **Extension config should be minimal** - only use documented fields from [Zed docs](https://zed.dev/docs/extensions/languages)
- **Reference working extensions** - [Erlang](https://github.com/zed-extensions/erlang), [Elixir](https://github.com/zed-extensions/elixir) are good examples

### Debugging Methodology
- **Log-based diagnosis** - `/tmp/sc_launcher_stdin.log` revealed absence of requests
- **Comparative analysis** - comparing with working extensions identified the config difference
- **Client vs server separation** - distinguishing where in the pipeline issues occur

### Tree-Sitter vs LSP
- Tree-sitter is for syntax highlighting only
- `textobjects.scm` is for Vim mode navigation, not LSP features
- Go-to-definition is purely LSP-based

## Next Steps

### Immediate Testing (Priority 1)
Systematically verify all LSP capabilities now that navigation works:

- [x] `textDocument/definition`
- [x] `textDocument/references`
- [ ] `textDocument/hover` - hover documentation (provider added 2026-01-06)
- [ ] `textDocument/completion` - autocomplete
- [ ] `textDocument/signatureHelp` - parameter hints
- [ ] `textDocument/documentSymbol` - outline view (rehydration added 2026-01-06)
- [ ] `textDocument/codeLens` - code lens (was timing out before)
- [ ] `workspace/symbol` - global symbol search

**Test edge cases:**
- Cross-file navigation
- Built-in class library definitions
- Large files (>1000 lines)
- Multiple definition results

### Outstanding Issues (Priority 2)

#### Custom Notification Warning
**Issue:** `Language server with id X sent unhandled notification supercollider/serverStatus`

**Options:**
- Remove custom notification
- Switch to standard LSP progress notifications (`window/workDoneProgress`)
- Document whether Zed extensions can handle custom notifications

**File:** Search for `supercollider/serverStatus` in quark code

#### CodeLens Reliability
**Issue:** Previously saw "Code Lens via supercollider failed: Server reset the connection"

**Action:** Monitor if this still occurs after config fix

### Performance Investigation (Priority 3)

**Questions to answer:**
- How long does LSPDatabase indexing take for large class libraries?
- What's the p95 latency for definition lookups?
- Are there bottlenecks in the 3-layer architecture (Zed → Launcher → sclang)?

**Approach:**
```supercollider
// Add timing instrumentation
var start = Main.elapsedTime;
var result = LSPDatabase.findDefinitions(...);
"Definition lookup: %ms".format((Main.elapsedTime - start) * 1000).postln;
```

### Architecture Questions (Priority 4)

**Current architecture:** Zed Extension (WASM) ↔ Launcher (Rust) ↔ sclang (UDP)

**Questions:**
1. Is the 3-layer bridge optimal, or could sclang communicate via stdio directly?
2. Should we bundle the launcher binary with the extension?
3. Can we reduce the ~2-3 second startup time?

**Research needed:** Study how other language servers handle similar scenarios

### Documentation (Ongoing)

**User docs needed:**
- Setup guide for the extension
- Feature overview (what LSP features are available)
- Troubleshooting common issues

**Developer docs needed:**
- Architecture diagram showing message flow
- Guide for adding new LSP features
- Explanation of capability negotiation

**Community contribution:**
- Document the config field gotcha for other extension authors
- Consider submitting findings to Zed discussions
- Prepare extension for Zed marketplace submission

### Future Enhancements

See `IMPROVEMENTS.md` for detailed list including:
- Enhanced navigation features (go to superclass, find subclasses)
- Integration features (scsynth status in status bar)
- Configuration options (custom sclang path, debug mode)
- Testing infrastructure

## Follow-up (2026-01-06)
- Fixed pending `didOpen` handling (string equality), caching last `didOpen` per URI.
- Providers (definition/references/documentSymbol/hover) now rehydrate docs from cached `didOpen` when `doc.isOpen`/`doc.string` is missing.
- Added a basic `HoverProvider` (returns symbol as markdown); outline rehydrates doc before building symbols.
- Logging temporarily at `info` to validate hydration; reduce after verification.

## Debugging Tools

```bash
# Monitor LSP requests from Zed
tail -f /tmp/sc_launcher_stdin.log

# Check sclang output and errors
tail -f /tmp/sclang_post.log

# Check Zed internal logs
tail -f ~/Library/Logs/Zed/Zed.log | grep -i supercollider

# Search for specific request types
grep -i "definition\|references\|hover\|completion" /tmp/sc_launcher_stdin.log

# Force restart everything
pkill -9 sc_launcher; pkill -9 sclang

# Rebuild extension
cd server/launcher && cargo build --release
# Then: Cmd+Shift+P → "zed: reload extensions"
```

## Resources

- [Zed Language Extensions](https://zed.dev/docs/extensions/languages)
- [LSP Specification](https://microsoft.github.io/language-server-protocol/)
- [Zed Extension Examples](https://github.com/zed-extensions/)
- [Extension Lifecycle Blog](https://zed.dev/blog/zed-decoded-extensions)

## Status Summary

| Component | Status | Notes |
|-----------|--------|-------|
| Go-to-definition | ✅ Working | Config fix resolved |
| Find references | ✅ Working | Config fix resolved |
| Server stability | ✅ Fixed | No more crashes |
| Extension config | ✅ Corrected | Removed invalid fields |
| Other LSP features | ⏳ Needs testing | Systematic verification needed |
| Custom notifications | ⚠️ Warning spam | Non-blocking, should fix |
| Documentation | 📝 In progress | User/dev docs needed |

## Follow-up (2026-01-06)

- Resolved TextDocumentProvider DNUs by giving its classvars accessors and initializing them in `*initClass`.
- Fresh launcher + initialize smoke test now runs cleanly (no DNU) and returns capabilities; see new `/tmp/sclang_post.log`.
- Next: rebuild the Zed extension and rerun the in-editor test sequence to confirm navigation and references stay healthy with the new provider queueing.
