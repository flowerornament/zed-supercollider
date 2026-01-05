# ADR-002: Minimal config.toml

Date: 2026-01-05
Status: Accepted

## Context

Go-to-definition and find references were not working in the extension.

## Problem

Log analysis revealed Zed wasn't sending LSP requests to the server. The config.toml contained undocumented fields:
```toml
opt_into_language_servers = ["supercollider"]
scope_opt_in_language_servers = ["supercollider"]
```

These fields appear in built-in Zed language configs but are not documented for extensions. Comparison with working extensions (Erlang, Elixir) showed they don't use these fields.

## Decision

Use minimal config with only documented fields:
```toml
name = "SuperCollider"
grammar = "supercollider"
path_suffixes = ["sc", "scd"]
line_comments = ["// "]
tab_size = 4
hard_tabs = false
```

## Rationale

Fields that work for built-in Zed languages break extension-provided languages.

**Verification:**
- Before: `grep -i "definition" /tmp/sc_launcher_stdin.log` showed zero requests
- After: Multiple `textDocument/definition` and `textDocument/references` requests appearing
- Result: Navigation working in editor

## Consequences

Navigation (go-to-definition, references) now works. Config matches documented Zed extension API and follows pattern of working extensions.

**Key learning:** Extension configs ≠ built-in configs. Always compare with working extensions, check logs for absence not just errors.

## References

- [Zed Language Extension Docs](https://zed.dev/docs/extensions/languages) - Official documented fields
- [Erlang Extension](https://github.com/zed-extensions/erlang), [Elixir Extension](https://github.com/zed-extensions/elixir) - Working reference examples
