---
description: Sync bd, stage all changes, commit with good message, push to origin
---

Commit and push all outstanding changes to origin/main.

## Steps

1. Run `bd sync` to capture any task changes
2. Run `git status` to see what's changed
3. Run `git diff --stat` to understand scope

### Handle Submodules (if any changes detected)

4. Check for submodule changes: `git submodule status`
   - Look for `+` prefix (submodule has new commits) or modified content
5. For each changed submodule:
   - `cd <submodule-path>`
   - `git status` to see submodule changes
   - `git add -A && git commit -m "<message>"`
   - `git push` (submodule must be pushed BEFORE parent)
   - `cd -` back to parent

### Commit Parent Repo

6. Stage all changes: `git add -A`
7. Write a commit message:
   - First line: 50 chars max, summarizes the "what"
   - Use conventional prefixes: feat/fix/chore/docs/refactor
   - Reference bd task IDs if relevant (e.g., "feat(m78.4): quiet logging")
   - If submodule updated, mention it (e.g., "chore: update LanguageServer.quark")
8. Commit (never --amend unless explicitly told)
9. Push to origin

## Rules

- NEVER force push
- NEVER amend commits that have been pushed
- If commit fails (hooks), fix and create NEW commit
- Always push submodules BEFORE parent repo
- End message with: 🤖 Generated with Claude Code
