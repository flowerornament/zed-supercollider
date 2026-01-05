# Prompt: Pick and Execute a Task

Use this when starting a work session to choose a task from `.ai/tasks/` and make meaningful progress while keeping the worktree clean.

## Steps
1) Load context:
   - `.ai/context.md` (state, anti-patterns)
   - `.ai/tasks/2026-01-05-execution-plan.md` plus relevant task file(s) (check YAML front matter: priority/status/updated).
2) Select a task:
   - Prefer highest-priority `status: active` (P0 > P1 > ...).
   - If none active, propose activating the top backlog item with rationale.
3) Plan briefly (2–4 bullets) and keep scope tight.
4) Work the task:
   - Follow conventions/anti-patterns; use vendored quark only.
   - Run focused checks (e.g., `scripts/validate-config.sh`) as needed.
5) Update task doc:
   - Bump `updated` in front matter.
   - Add a `## Status Log` entry with date, progress, decisions, evidence.
   - Adjust `status` if appropriate (e.g., `active` → `done` with verification).
6) Keep README user-facing; doc notes live in `.ai/`.
7) Finish clean:
   - Ensure `git status` is clean or explain remaining deltas.
   - Summarize changes, tests, and next suggested steps.

## What made prior sessions good
- Clear task selection from YAML metadata; explicit status updates.
- Minimal noise defaults (launcher logging now opt-in); configs validated via script.
- README kept user-focused; contributor notes centralized in `.ai/`.
