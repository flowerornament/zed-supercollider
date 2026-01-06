title: "AI Agent Guide"
created: 2026-01-05
updated: 2026-01-07
purpose: "Small, consistent entrypoint for AI agents working on the SuperCollider Zed extension"
---

# AI Agent Guide

Read this file, then jump to `context.md`. Keep the docs lean; update them when reality changes.

## Minimal Map
- `context.md` (start here): purpose, status, anti-patterns, quick workflow.
- `architecture.md`: one-page mental model of the dual-channel bridge.
- `conventions.md`: only the rules that keep SC/Rust/Zed code safe.
- `commands.md`: the few commands that prove things work or fail.
- `tasks/2026-01-05-execution-plan.md`: single backlog; front matter tracks status.
- `decisions/`: ADRs that justify the architecture/config choices.
- `prompts/`: short checklists for debugging or picking work.
- `research/`: past investigations worth reusing.
- `code-reviews/`: last notable review findings.

## Permissions Note
- Some scripts (e.g., `scripts/check-setup.sh`, `scripts/test-sclang.sh`) need to launch the system sclang at `/Applications/SuperCollider.app/Contents/MacOS/sclang`. In sandboxed runs this can fail with a NEON/processor error. If that happens, rerun with escalated permissions so the script can access the system binary.

## How to Work
- Read `context.md`, then open the active task in `tasks/`.
- Obey anti-patterns and conventions; do not reintroduce banned config fields or SC `^` returns.
- When you change state or learn something new, update the relevant doc (keep it short).
- Record progress in the task front matter + `## Status Log`.
- Prefer evidence: link to logs/commands you ran; avoid narrative.

## When Adding Docs
- Keep front matter (`title/date/updated/priority/status/owners/purpose`) and a brief log.
- Research: `.ai/research/YYYY-MM-DD-topic.md` with problem/decision/evidence.
- Decisions: `.ai/decisions/NNN-topic.md` with context → decision → consequence.
- Prompts: only checklists that save time for recurring workflows.
