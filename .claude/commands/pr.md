---
description: Create a PR with staged changes for review
---

Create a pull request for outstanding changes.

## Steps

1. Run `bd sync` to capture any task changes
2. Run `git status` to see what's changed
3. Run `git diff --stat` to understand scope
4. If on main, create a feature branch:
   - Name format: `<type>/<short-description>` (e.g., `fix/pid-tracking`)
   - Use: `git checkout -b <branch-name>`
5. Stage all changes: `git add -A`
6. Write a commit message:
   - First line: 50 chars max, summarizes the "what"
   - Use conventional prefixes: feat/fix/chore/docs/refactor
   - Reference bd task IDs if relevant (e.g., "feat(m78.4): quiet logging")
7. Commit (never --amend unless explicitly told)
8. Push branch: `git push -u origin <branch-name>`
9. Create PR using gh CLI:
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
10. Return the PR URL

## Rules

- NEVER push directly to main (that's what /commit is for)
- NEVER force push
- If on a feature branch already, commit and push there
- Keep PRs focused - one logical change per PR
- End PR body with: 🤖 Generated with Claude Code
