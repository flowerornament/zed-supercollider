---
title: "Prompt: Pick and Execute a Task"
created: 2026-01-05
updated: 2026-01-05
purpose: "Workflow template for starting a work session by selecting and executing tasks from the backlog"
---

# Prompt: Pick and Execute a Task

Use this to start a work session by choosing a task from `.ai/tasks/` and getting moving.

## Steps
1) Read `.ai/context.md` and `.ai/tasks/2026-01-05-execution-plan.md` plus the relevant priority file (check YAML front matter for priority/status).
2) Choose the highest-priority `status: active` item (P0 > P1 > P2 > P3). If none active, propose activating the top backlog item.
3) Make a short plan (2–4 bullets) and scope tightly.
4) Do the work, following conventions/anti-patterns; run only the checks needed for the task.
5) Update the task file: bump `updated`, add a `## Status Log` entry with date/progress/decisions/evidence, and adjust `status` if needed.
6) End with a clean `git status` or explain any intentional deltas, and summarize changes/tests/next steps.
