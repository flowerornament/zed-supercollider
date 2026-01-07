---
description: Create a PR with staged changes for review
---

Create a pull request for outstanding changes.

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

### Create Branch and Commit

6. If on main, create a feature branch:
   - Name format: `<type>/<short-description>` (e.g., `fix/pid-tracking`)
   - Use: `git checkout -b <branch-name>`
7. Stage all changes: `git add -A`
8. Write a commit message:
   - First line: 50 chars max, summarizes the "what"
   - Use conventional prefixes: feat/fix/chore/docs/refactor
   - Reference bd task IDs if relevant (e.g., "feat(m78.4): quiet logging")
   - If submodule updated, mention it (e.g., "chore: update LanguageServer.quark")
9. Commit (never --amend unless explicitly told)
10. Push branch: `git push -u origin <branch-name>`

### Create PR

11. Create PR using gh CLI:
   ```
   gh pr create --title "<commit first line>" --body "$(cat <<'EOF'
   ## Summary
   <bullet points of changes>

   ## Test plan
   - [ ] <verification steps>

   🤖 Generated with [Claude Code](https://claude.com/claude-code)
   EOF
   )"
   ```
12. Return the PR URL

## Rules

- NEVER push directly to main (that's what /commit is for)
- NEVER force push
- If on a feature branch already, commit and push there
- Keep PRs focused - one logical change per PR
- Always push submodules BEFORE parent repo
- End PR body with: 🤖 Generated with Claude Code
