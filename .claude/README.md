# Claude Code Configuration

This directory contains custom skills and configuration for working with Claude Code in this repository.

## Skills

### `/smart-commit` - Intelligent Commit/PR Decision

Automatically decides whether to create a Pull Request or commit directly based on change significance.

**Usage:**
```bash
/smart-commit
```

**Decision Criteria (ANY of these triggers a PR):**
- 10+ files changed
- 100+ lines changed (additions + deletions)
- Core files modified (see `.claude-pr-policy.json`)
- Commit message indicates new feature

**Configuration:**
Edit `../.claude-pr-policy.json` to customize thresholds and core files list.

**Manual Override:**
- `/pr` - Force PR creation regardless of thresholds
- `/commit` - Force direct commit/push regardless of thresholds

## Configuration Files

### `../.claude-pr-policy.json`

Defines when PRs are required vs. direct commits.

```json
{
  "thresholds": {
    "files_changed": 10,      // Number of modified files
    "lines_changed": 100,     // Total additions + deletions
    "core_files": [           // Files that always require PR
      "server/launcher/src/main.rs",
      "Cargo.toml",
      // ... add more critical files
    ]
  },
  "pr_required_if": "any"     // "any" (OR logic) or "all" (AND logic)
}
```

**Customization:**
- Increase thresholds to create fewer PRs (more direct commits)
- Decrease thresholds to create more PRs (more review)
- Add files to `core_files` to always require PR review for critical changes
- Change `pr_required_if` to "all" to require ALL conditions (stricter)

## Git Workflow

**Branches:**
- `main` - Stable, verified code only
- `dev` - Working branch, all `/commit` pushes go here
- `beads-sync` - Issue tracking (auto-managed)

**Why dev?** Multiple Claude Code instances may work in parallel. Pushing to `dev` prevents incomplete work from landing on `main`.

**Commands:**
- `/commit` - Commit and push to `dev` branch
- `/pr` - Create PR targeting `main`
- `/release` - Merge verified `dev` to `main`

## Workflow

When you complete work:

1. Run quality checks (tests, linters, builds)
2. Close/update issues with `bd close <id>` or `bd update <id>`
3. Run `/commit` to push to dev
4. Verify with `git log --oneline origin/dev -3`

To release verified work to main:
```bash
/release
```
