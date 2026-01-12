title: "Code Conventions & Patterns"
created: 2026-01-05
updated: 2026-01-07
purpose: "Only the rules that keep the SuperCollider/Rust/Zed code safe and predictable"
---

# Code Conventions & Patterns

## SuperCollider (quark)
- Handle nils: `dict[key] ?? { Array.new }` before `.add`. Never assume keys exist.
- Classvars: initialize in `*initClass`; expose accessors as needed.
- Dictionary functions: never use `^` (non-local return). Use expression returns; otherwise `valueArray` returns the provider, not your dict.
- Races: accept `didChange` before `didOpen`; queue or mark open rather than throwing.
- Logging: include context and use real levels (`info`/`warning`/`error`); avoid noisy `postln`.

## Zed extension (WASM)
- Config minimal: only documented fields in `languages/SuperCollider/config.toml`; adding `opt_into_language_servers` or `scope_opt_in_language_servers` breaks navigation.
- Dev launcher: only prefer the local binary when it exists. Otherwise honor user settings/PATH.
- Settings: deep-merge user overrides with defaults; do not clobber nested maps.

## Launcher (Rust)
- Buffer LSP until `"***LSP READY***"`; flush on ready and on graceful shutdown.
- Chunk UDP (~6KB) and surface oversize/UDP failures as HTTP errors instead of pretending success.
- Treat stdin-close as shutdown; kill only the child sclang you spawned, not global processes.
- Keep logging level-gated; default quiet, opt-in debug logs under TMP.

## Tree-sitter / runnables
- Add tags in `runnables.scm` (`#set! tag sc-eval`) so tasks can match; without tags, play buttons won't bind.
- Captures are case-sensitive: `@code` → `$ZED_CUSTOM_code` (not uppercase).
- Extension tasks go in `languages/SuperCollider/tasks.json`, not `.zed/tasks.json`.
- Keep highlight queries precise; avoid broad `(identifier) @variable` captures that make everything the same color.

## Testing/verification habit
- Navigation working: `grep -i "definition" /tmp/sc_launcher_stdin.log`.
- Eval working: `curl -X POST -d "1 + 1" http://127.0.0.1:57130/eval`.
- Quark clean: `grep -i "error\\|exception\\|dnu" /tmp/sclang_post.log`.
- Reset before retesting major changes: `pkill -9 sc_launcher sclang; rm -f /tmp/sc_launcher_stdin.log /tmp/sclang_post.log`.
