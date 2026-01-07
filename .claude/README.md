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

## Workflow

When you complete work:

1. Run quality checks (tests, linters, builds)
2. Close/update issues with `bd close <id>` or `bd update <id>`
3. Run `/smart-commit` to automatically commit or create PR
4. Verify with `git status`

Claude Code will automatically:
- Analyze your changes
- Show you why a PR was or wasn't created
- Execute the appropriate workflow
- Ensure changes are pushed to remote

## Examples

**Small bug fix (direct commit):**
```
$ /smart-commit
Analyzing changes...
- Files changed: 2
- Lines changed: 15 additions, 3 deletions (18 total)
- Core files modified: none

✓ Changes are minor. Committing directly to main...
```

**Large refactoring (PR created):**
```
$ /smart-commit
Analyzing changes...
- Files changed: 8
- Lines changed: 201 additions, 119 deletions (320 total)
- Core files modified: server/launcher/src/main.rs

✓ Threshold exceeded: lines_changed (320 > 100)
✓ Threshold exceeded: core_files (main.rs is critical)

📋 Changes are significant. Creating PR for review...
```
