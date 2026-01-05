---
title: "Execution Plan – Stability, Quality, and Delivery (Unified)"
date: 2026-01-05
priority: umbrella
status: active
owners: [team]
updated: 2026-01-05
---

# Execution Plan – Stability, Quality, and Delivery (Unified)

Single source of truth for finishing the SuperCollider Zed extension with high code quality, consistency, robustness, and maintainability. Merges the best items from `improvements.md`, `improvements-detailed.md`, and prior low-hanging fixes.

## Status Log
- 2026-01-05: Status set to active; focus is P0 items before moving to later phases.

## Goal
Ship a stable, portable extension that avoids user-facing regressions/noise, aligns advertised capabilities with reality, and lays groundwork for performance and future features.

## Scope
- Launcher (`server/launcher/src/main.rs`)
- Extension entry (`src/lib.rs`)
- Vendored quark (`server/quark/LanguageServer.quark/...`)
- Language config and tasks (`languages/SuperCollider/config.toml`, `.zed/tasks.json`, scripts)
- Docs/CI guardrails

## P0 – Immediate Fixes (ship first)
- **Remove custom notification spam:** Drop `serverStatus` capability in `create_initialize_response`; disable/remove `Notifications/ServerStatusNotification.sc`; confirm no remaining senders. Goal: no “unhandled notification supercollider/serverStatus” in Zed logs.
- **Dev launcher selection:** Only select dev `server/launcher/target/release/sc_launcher` when it exists; log clearly; keep settings/PATH precedence in `language_server_command` and slash command.
- **Safe startup/shutdown:** Remove global `pkill -9 sclang`; add scoped shutdown (LSP shutdown/exit, wait with timeout, TERM/kill child PID only if needed); ensure stdin close triggers clean exit. Update emergency task to TERM-first or remove.
- **Capability hygiene:** Advertise only what works (likely drop `documentSymbolProvider`, on-type formatting, `serverStatus`; keep selectionRange/folding/codeLens only if providers return data; formatting toggle respected). Align `DocumentSymbolProvider` skip logic with advertised state.
- **Probe JSON correctness:** Use `serde_json::json!` + `to_string()` for `--mode probe`; handle quotes/backslashes safely.
- **Logging defaults:** Minimal by default; add env/flag for verbose file logs; prefer `temp_dir` over hardcoded `/tmp`; avoid duplicate writes unless debug-enabled.
- **Tasks cleanup:** Inline eval curl (no absolute paths), add HTTP status output, allow configurable port/env, soften kill task (TERM-first, no global nukes), remove/replace `.zed/eval.sh` if unnecessary.
- **Config guardrails:** Add `scripts/validate-config.sh` to fail on banned keys (`opt_into_language_servers`, `scope_opt_in_language_servers`, etc.) and missing required fields; wire into pre-commit/CI stub.
- **Error messaging polish:** Actionable guidance when launcher/quark missing; check-setup uses `status.success()`; include sample settings JSON and quark install hints; log when dev binary is absent.
- **Quark safety:** `LSPDatabase.sc:renderMethodRange` try/finally close; coerce `includeDeclaration` to Bool in `FindReferencesProvider`; gate/remove DEBUG `postln` (Hover/Refs/DocumentSymbol).
- **Carryover quick wins:** Add `word_characters = ["a-zA-Z0-9_?"]` (if desired); README setup/usage/troubleshooting plus fire-and-forget eval note.

## P1 – Short-Term Hardening (after P0)
- **Graceful shutdown polish:** Flush buffered messages on shutdown; ensure only child sclang is killed; avoid message loss; log shutdown timing.
- **LSP correctness:** Proper null/empty responses; prefer `LocationLink` for definitions; dedupe initialize responses robustly; handle partial UDP sends gracefully.
- **Logging/tracing framework:** Replace ad-hoc `eprintln!` with structured logging (`tracing`/`log`), consistent prefixes, gated previews, temp_dir logs.
- **Provider alignment:** Only advertise capabilities with real behavior; if enabling formatting, wire it; otherwise keep off; reduce stub returns.
- **Task/HTTP ergonomics:** Document async eval; optional port env var; note CORS/security expectations.

## P2 – Performance & Quality
- **Indexing/cache:** LSPDatabase LRU cache and incremental/index invalidation on recompile; cache common definitions/responses.
- **Message pipeline:** Batch small UDP messages; centralize magic numbers (timeouts/chunk sizes); adopt type-safe `lsp-types` handling to reduce JSON churn.
- **Startup optimization:** Measure and reduce fixed sleeps; capture startup timings.
- **Testing/benchmarks:** Add benchmarks for definition lookup; set and track targets (startup <2s, definition <100ms p95, steady memory <50MB).

## P3 – Features & DX (when stable)
- **Progress notifications:** Standard `$ /progress` for indexing.
- **Signature help and workspace symbols:** Harden parsing; add fuzzy search.
- **Provider robustness:** Input validation layer for params; consistent logging via `Log()` in quark.
- **Hot reload (optional):** Dev-only quark reload helper.
- **Docs/CI:** Architecture/docs polish, config schema, CI pipeline (launcher tests, validate-config, fmt/clippy), release automation, integration tests (ignored if no sclang).

## Validation Checklist
- Launcher: `--mode lsp` initialize response matches advertised capabilities; no `serverStatus`; dev binary auto-picked only when present; ENOENT-free on fresh clone. `--mode probe` returns valid JSON with quotes/backslashes.
- Tasks: Eval works on fresh clone with no path edits; HTTP status visible; kill task is safe and targeted; port configurable.
- Config: `scripts/validate-config.sh` passes current tree; fails when banned keys added; hook runs in CI/pre-commit.
- Quark: No FD leaks; references handle `includeDeclaration` safely; debug logs quiet unless enabled.
- Shutdown: Closing stdin or invoking shutdown cleans child sclang without touching unrelated processes; pending messages flushed.
- Docs: README updated with setup/usage/troubleshooting and async eval note; language config includes desired `word_characters`.

## Guardrails / Anti-Patterns
- Keep `languages/SuperCollider/config.toml` minimal; never add `opt_into_language_servers` or `scope_opt_in_language_servers`.
- Avoid `^` returns in SC dictionary functions; initialize classvars in `*initClass`; handle nil dictionary keys.
- Do not overwrite user-installed quark without consent; test vendored copy first.
- HTTP eval is fire-and-forget; results appear in Post Window.
