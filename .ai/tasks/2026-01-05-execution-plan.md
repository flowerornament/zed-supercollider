title: "Execution Plan – Stability, Quality, and Delivery"
created: 2026-01-05
updated: 2026-01-06
priority: umbrella
status: active
owners: [team]
purpose: "Single roadmap (P0–P2) to ship a stable, Zed-friendly SuperCollider extension with accurate capabilities and reliable eval/control."
---

# Execution Plan – Stability, Quality, and Delivery (Unified)

This is the single backlog (P0–P2). Update `updated` + `## Status Log` when you touch anything.

## Status Log
- 2026-01-06: Reviewed live code vs docs. Extension still forces `SC_LAUNCHER_DEBUG*` env by default (logs noisy), HTTP `/eval` returns 202 status only, and launcher initialize advertises extra capabilities (signature/folding/selection/workspaceSymbol/codeLens) despite limited support. CORS remains `*`, no HTTP body limits yet.
- 2026-01-07: Simplified plan and docs; kept P0 list intact.
- 2026-01-06: Collapsed tasks into this single plan (P0–P2); LSP debugging file marked done; removed redundant task files.
- 2026-01-05: Status set to active; focus is P0 items before moving to later phases.

## Goal
Ship a stable, portable extension whose advertised capabilities match reality, with safe eval/control and quiet defaults.

## Scope
Launcher (`server/launcher/src/main.rs`), extension (`src/lib.rs`), quark (`server/quark/LanguageServer.quark`), language config/tasks (`languages/SuperCollider`, `.zed/tasks.json`, scripts), docs/CI guardrails.

## P0 – Ship-Blockers
- Capability hygiene: drop `serverStatus`; advertise only working providers (definition/references/completion/hover/executeCommand).
- Init/shutdown: clean stdin-close handling; buffer until ready, then flush and shut down without orphaning or global kills.
- Eval/control transport: enforce UDP-safe size (chunk or 413), surface UDP failures as 4xx/5xx; keep HTTP localhost-only with clear status.
- Logging: default quiet; opt-in debug logs under TMP; stop forcing debug env vars.
- Tasks/PIDs: inline curl tasks with status output; track launcher+sclang PIDs or remove any global `pkill` task.
- Dev launcher selection: settings path > PATH > dev binary (only if built); clear logs when dev binary missing.
- Packaging/tests/meta: remove committed build artifacts, fix `extension.toml` metadata, add minimal launcher tests + CI stub (`scripts/validate-config.sh` + tests).

## P1 – Hardening
- Flush buffered UDP/LSP responses on shutdown; log phases/timing.
- Normalize responses (null/empty), prefer `LocationLink`, dedupe initialize responses.
- Structured logging gated by env; separate user-facing post log from debug logs.
- Document async eval semantics and loopback/CORS expectations.

## P2 – Performance/Quality
- Small LRU cache for defs/refs with invalidation; light hit/miss metrics.
- Centralize UDP constants; use serde/lsp-types to cut JSON churn.
- Measure startup and replace sleeps with readiness signals; add tiny benches for navigation latency.

## Validation checklist
- Initialize response aligns with capabilities; no `serverStatus`; dev binary selection doesn’t throw ENOENT.
- Eval/control tasks return real HTTP status; kill task (if kept) targets only tracked PIDs.
- `scripts/validate-config.sh` + launcher tests pass in CI stub.
- Shutdown closes stdin, sends LSP shutdown/exit, and leaves no orphaned `sclang`/`sc_launcher`.
- README/setup match logging defaults and eval semantics.

## Guardrails
- `languages/SuperCollider/config.toml` stays minimal (no `opt_into_language_servers`/`scope_opt_in_language_servers`).
- SC dict funcs never use `^`; init classvars in `*initClass`; handle nil dict keys.
- Do not overwrite user-installed quark without consent; test vendored copy first.
- HTTP eval is fire-and-forget; results land in Post Window/logs, not inline.
