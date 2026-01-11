title: "Prompt: Pick and Execute a Task"
created: 2026-01-05
updated: 2026-01-07
purpose: "Tiny workflow for starting a session and leaving a clean handoff"
---

# Prompt: Pick and Execute a Task

1) Read `context.md` + `tasks/2026-01-05-execution-plan.md`. Grab the highest-priority `status: active` (P0→P2). If none, propose one P0 from backlog.
2) Write a 2–3 bullet plan; keep scope narrow.
3) Do the work following conventions/anti-patterns; run only the checks that prove your change.
4) Update the task file: bump `updated`, add a dated bullet under `## Status Log`, adjust `status` if it moved.
5) Finish with clean `git status` or note intentional leftovers; summarize changes/tests/next steps.
