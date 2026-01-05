---
title: "Performance & Quality (P2)"
date: 2026-01-05
priority: P2
status: backlog
owners: [team]
updated: 2026-01-05
---

# Performance & Quality (P2)

Targets to improve speed, memory, and overall efficiency after stability is proven. Keep measurements alongside changes.

## Status Log
- 2026-01-05: Status set to backlog; revisit after P0/P1 tasks.

## Scope & Link
- Performance/quality improvements after stability. See umbrella: `.ai/tasks/2026-01-05-execution-plan.md`.

## Indexing/Cache
- Add LSPDatabase LRU cache for definitions/references; size-bound and eviction-aware.
- Invalidate cache on recompile, file change, or relevant workspace events; hook into providers that mutate state.
- Track cache hit/miss metrics and memory usage; expose lightweight diagnostics.
- Verification: benchmark lookup latency before/after; confirm invalidation on recompile and file edits.

## Message Pipeline
- Centralize UDP constants (chunk size, timeouts, retry intervals) and expose configuration knobs for dev/testing.
- Batch small UDP messages when safe to reduce syscalls; ensure ordering is preserved.
- Adopt type-safe `lsp-types` or structured serde models to reduce string churn and improve validation.
- Verification: simulate large responses/fragmentation; measure packet counts and error rates.

## Startup Optimization
- Measure startup timing phases (launcher start, sclang spawn, LSP READY, first response).
- Replace fixed sleeps with readiness signals or timeouts; minimize latency to first response.
- Stretch targets (optional): startup <2s, definition <100ms p95, steady memory <50MB; treat as aspirational until measured.
- Verification: collect timing logs; repeat runs to confirm consistency.

## Testing/Benchmarks
- Add benchmarks for definition/reference lookup and startup; keep CI-fast via feature flags/ignored benches.
- Create scripts to profile memory/CPU during typical workflows; capture baselines.
- Verification: run benches locally; include instructions to reproduce profiles.
