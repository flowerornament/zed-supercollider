# ADR-002: Minimal config.toml (Remove opt_into_language_servers)

**Date:** 2026-01-05
**Status:** Accepted
**Deciders:** Investigation of navigation failure

## Context

Go-to-definition and find references were not working. Cmd+click and Command Palette "Go to Definition" did nothing.

## Problem Discovery

Investigation revealed:
1. Server was healthy, advertising capabilities correctly
2. Server logs showed no errors, providers registered successfully
3. **Critical finding:** `/tmp/sc_launcher_stdin.log` showed ZERO `textDocument/definition` requests
4. Conclusion: Client-side issue - Zed wasn't sending requests to server

## Investigation Process

### Step 1: Comparison with Working Extensions
Compared with Rust extension (known working):
- Rust: Shows status bar activity, go-to-definition works
- SuperCollider: No status bar activity, no definition requests

### Step 2: Config Analysis
Examined `languages/SuperCollider/config.toml`:
```toml
name = "SuperCollider"
grammar = "supercollider"
path_suffixes = ["sc", "scd"]
line_comments = ["// "]
tab_size = 4
hard_tabs = false
opt_into_language_servers = ["supercollider"]  # 🚨 Suspicious
scope_opt_in_language_servers = ["supercollider"]  # 🚨 Suspicious
```

Checked working extension ([Erlang](https://github.com/zed-extensions/erlang/blob/main/languages/erlang/config.toml)):
```toml
name = "Erlang"
grammar = "erlang"
path_suffixes = ["erl", "hrl", ...]
line_comments = ["% ", "%% ", "%%% "]
# NO opt_into_language_servers field
# NO scope_opt_in_language_servers field
```

### Step 3: Research
- Fields not documented in [Zed extension docs](https://zed.dev/docs/extensions/languages)
- Found in some built-in Zed configs (TSX)
- Hypothesis: Work for built-in languages, break extensions

## Decision

Remove `opt_into_language_servers` and `scope_opt_in_language_servers` from config.toml.

Use minimal config with only documented fields:
```toml
name = "SuperCollider"
grammar = "supercollider"
path_suffixes = ["sc", "scd"]
line_comments = ["// "]
tab_size = 4
hard_tabs = false
```

## Verification

**Before removal:**
```bash
grep -i "definition" /tmp/sc_launcher_stdin.log
# (empty - zero requests)
```

**After removal:**
```bash
grep -i "definition" /tmp/sc_launcher_stdin.log
# {"jsonrpc":"2.0","id":4,"method":"textDocument/definition","params":...}
# {"jsonrpc":"2.0","id":6,"method":"textDocument/definition","params":...}
# {"jsonrpc":"2.0","id":8,"method":"textDocument/definition","params":...}
```

Also started receiving:
- `textDocument/references` requests ✅
- Server responding correctly ✅
- Navigation working in editor ✅

## Why These Fields Break Extensions

**Hypothesis:** Different code paths in Zed for built-in vs extension-provided languages.

**Evidence:**
- These fields appear in built-in language configs (TSX, JavaScript)
- Working extensions (Erlang, Elixir, Gleam) don't use them
- Removing them fixed the issue immediately

**Likely mechanism:**
- Built-in languages: Fields control which language servers to enable for that language
- Extension languages: Extension already specifies language server, fields may disable features
- Exact mechanism unknown (would require Zed source code analysis)

## Consequences

### Positive
- ✅ Navigation working (go-to-definition, find references)
- ✅ Config matches documented Zed extension API
- ✅ Simpler, more maintainable config
- ✅ Follows pattern of working extensions

### Negative
- None identified

### Neutral
- Need to document this gotcha for other extension authors
- Could submit PR to Zed docs warning about this

## Lessons Learned

1. **Extension configs ≠ Built-in configs:** Fields that work for built-in languages may break extensions
2. **Always compare with working extensions:** Don't assume all config fields are valid
3. **Check logs for absence, not just errors:** Zero requests is a symptom, not an error message
4. **Client vs server debugging:** Distinguish where in pipeline issues occur

## Implementation Checklist

- [x] Remove `opt_into_language_servers` from config.toml
- [x] Remove `scope_opt_in_language_servers` from config.toml
- [x] Verify navigation working (grep for definition requests)
- [x] Test go-to-definition in editor
- [x] Test find references in editor
- [x] Document in .ai/context.md anti-patterns
- [x] Document in RESEARCH.md

## Related Files

- `languages/SuperCollider/config.toml` - Where change was made
- `.ai/context.md` - Anti-pattern documented
- `.ai/research/2026-01-05-navigation.md` - Full investigation details

## Future Work

- Consider submitting issue to Zed about this behavior
- Add warning to Zed extension documentation
- Share findings in Zed community for other extension authors

## References

- [Zed Language Extension Docs](https://zed.dev/docs/extensions/languages) - Official documented fields
- [Erlang Extension Config](https://github.com/zed-extensions/erlang/blob/main/languages/erlang/config.toml) - Working reference
- [Elixir Extension Config](https://github.com/zed-extensions/elixir/blob/main/languages/elixir/config.toml) - Working reference
- [TSX Built-in Config](https://github.com/zed-industries/zed/blob/main/crates/languages/src/tsx/config.toml) - Where these fields appear
