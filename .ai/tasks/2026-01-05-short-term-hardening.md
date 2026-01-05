---
title: "Short-Term Hardening (P1)"
date: 2026-01-05
priority: P1
status: backlog
owners: [team]
updated: 2026-01-05
---

# Short-Term Hardening (P1)

Robustness and correctness improvements once P0 is complete. Expand only after immediate fixes are stable.

## Status Log
- 2026-01-05: Status set to backlog; queued behind P0 fixes.

## Graceful Shutdown Polish
- Flush buffered UDP/LSP messages before closing sockets; ensure pending responses are written to stdout.
- Track child PIDs explicitly; confirm only the launcher-spawned sclang/scsynth are terminated.
- Log shutdown phases (stdin closed → shutdown sent → TERM/KILL if needed) with timings; avoid repeating P0 behavior descriptions.
- Verification: orchestrate stdin-close/shutdown once; confirm no lost responses, no orphan processes.

## LSP Correctness
- Normalize null/empty responses per spec (e.g., `[]` vs `null`) across all providers.
- Prefer `LocationLink` for definitions where start/end ranges differ; retain `Location` fallback when identical.
- Harden initialize response deduping to avoid double-capability responses from sclang and launcher.
- Handle partial UDP sends/fragmentation gracefully: backoff/retry per chunk; detect and recover from partial headers.
- Verification: run definition/hover/references with synthetic tests; inspect logs for patched responses and missing duplicates.

## Logging/Tracing Framework
- Replace ad-hoc `eprintln!` with structured logging (`tracing`/`log`) and consistent prefixes.
- Gate verbose logging with env/flag; write files to `SC_TMP_DIR`/`TMPDIR` with rotation/size caps.
- Separate user-facing post window logs from debug logs; document env flags.
- Verification: enable tracing and confirm categories/levels; ensure defaults stay quiet.

## Provider Alignment
- Advertise only capabilities with real behavior; disable formatting unless wired end-to-end.
- Reduce stub returns; add guards against nil/DNU in all providers.
- Add lightweight sanity tests (unit-style for SC where feasible, Rust tests for launcher) to prevent regressions.
- Verification: capture initialize response and compare to registered providers; run provider-specific smoke checks.

## Task/HTTP Ergonomics
- Document async eval behavior and response expectations (202/“sent” semantics).
- Allow port/env overrides in docs and tasks; clarify CORS/security expectations (loopback only).
- Consider optional JSON response for eval (status + request id) while keeping fire-and-forget default.
- Verification: curl tasks return expected status; optional JSON path tested behind flag.
