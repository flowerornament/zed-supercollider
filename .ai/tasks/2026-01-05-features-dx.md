---
title: "Features & Developer Experience (P3)"
date: 2026-01-05
priority: P3
status: backlog
owners: [team]
updated: 2026-01-05
---

# Features & Developer Experience (P3)

Longer-term enhancements after stability/performance work. Prioritize based on user feedback and feasibility.

## Status Log
- 2026-01-05: Status set to backlog; activate after stability/performance work.

## Scope & Link
- Longer-term features/DX. See umbrella: `.ai/tasks/2026-01-05-execution-plan.md`.

## Progress Notifications
- Implement standard `$ /progress` for indexing/long-running operations; throttle to avoid log noise.
- Ensure client compatibility (Zed/VSCode) and provide clear titles/messages.
- Verification: simulate indexing; confirm progress events appear and complete cleanly.

## Signature Help & Workspace Symbols
- Harden parsing for signature help; include argument docs and types where available.
- Add fuzzy matching and scoring for workspace symbols; handle large workspaces efficiently.
- Verification: query signature help on common classes/methods; run workspace symbol search on large projects.

## Provider Robustness
- Add input validation layer for request params to prevent DNU/TypeError paths.
- Centralize logging via `Log()` in quark with consistent levels/tags; remove stray `postln`.
- Verification: fuzz params (missing fields, wrong types); ensure graceful handling and logged warnings.

## Hot Reload (Optional)
- Dev-only helper to reload quark without full sclang restart; guard behind explicit env/flag.
- Document workflow for contributors; ensure it never runs in production settings.
- Verification: manual dev runs; confirm reload works without orphaning state.

## Docs/CI
- Polish architecture docs and config schema; add release notes/checklist.
- Build CI pipeline: launcher tests, `scripts/validate-config.sh`, fmt/clippy, optional integration tests (ignored when sclang absent).
- Add integration tests for launcher↔quark flows when feasible; mark them optional/ignored without sclang.
