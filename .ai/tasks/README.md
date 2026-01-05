# Task Docs Conventions

Each file in `.ai/tasks/` starts with YAML front matter to track status and ownership, followed by a short status log for updates and decisions.

## Front Matter
Required keys:
```yaml
---
title: "Short description"
date: YYYY-MM-DD        # original creation date
priority: P0|P1|P2|P3|umbrella
status: active|backlog|blocked|done
owners: [team]          # list of owners (use team if shared)
updated: YYYY-MM-DD     # last touch
tags: [optional, tags]  # optional
---
```

## Status Values
- `active`: currently being worked.
- `backlog`: queued, not in progress.
- `blocked`: waiting on dependency/decision.
- `done`: complete; capture outcomes in the log.

## Status Log & Reporting
- Keep a `## Status Log` section in each file with dated bullets capturing progress, decisions made, and evidence/links when useful.
- When updating a task, bump `updated` in front matter and add a log entry. Example:
  - `2026-01-05: Set status to active; focusing on P0 capability hygiene. Evidence: /tmp/sc_launcher_stdin.log shows definition requests.`
- For completions, add the key decision(s), any follow-up tickets, and verification performed.
