---
description: Sync bd, stage all changes, commit with good message, push to origin
---

Commit and push all outstanding changes to origin/main.

## Steps

1. Run `bd sync` to capture any task changes
2. Run `git status` to see what's changed
3. Run `git diff --stat` to understand scope
4. Stage all changes: `git add -A`
5. Write a commit message:
   - First line: 50 chars max, summarizes the "what"
   - Use conventional prefixes: feat/fix/chore/docs/refactor
   - Reference bd task IDs if relevant (e.g., "feat(m78.4): quiet logging")
6. Commit (never --amend unless explicitly told)
7. Push to origin

## Rules

- NEVER force push
- NEVER amend commits that have been pushed
- If commit fails (hooks), fix and create NEW commit
- End message with: 🤖 Generated with Claude Code
