title: "LSP Navigation Research"
created: 2026-01-05
updated: 2026-01-07
purpose: "Short record of why navigation was broken and how it was fixed"
---

# LSP Navigation Research

## What broke
Navigation requests never left Zed; `/tmp/sc_launcher_stdin.log` had no `textDocument/definition` entries.

## Why
- `languages/SuperCollider/config.toml` used undocumented fields (`opt_into_language_servers`, `scope_opt_in_language_servers`) that stop Zed from sending LSP requests for extension languages.
- Quark also had nil-handling/race bugs that could crash early in startup.

## Fix
- Strip the undocumented config fields (keep only documented ones).
- Harden quark: init arrays before use, handle nil dict keys, queue `didOpen`/`didChange` until ready.

## Evidence/verification
- After config fix: `grep -i "definition" /tmp/sc_launcher_stdin.log` shows multiple requests.
- Hover/definition/references work; outline still absent because client never sends `textDocument/documentSymbol`.

## Takeaways
- Extension configs behave differently from built-ins—stick to documented fields and compare with known-good extensions (Erlang/Elixir).
- Always check for absence of requests in logs; missing traffic often means client-side config, not server bugs.
