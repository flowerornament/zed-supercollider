---
name: smart-commit
description: Intelligently decide whether to create a PR or commit directly based on change significance. Analyzes file count, line changes, and core files to recommend the best workflow. Use when completing work and need to commit/push changes.
allowed-tools: Bash, Read
---

# Smart Commit - Intelligent PR vs Direct Commit Decision

Automatically analyzes your staged changes and decides whether to:
1. **Create a Pull Request** (for significant changes requiring review)
2. **Commit and push directly** (for minor changes)

## Decision Criteria

This skill checks if **ANY** of these conditions are met (configurable in `.claude-pr-policy.json`):

- **Files changed** ≥ 10 files modified/added/deleted
- **Lines changed** ≥ 100 total (additions + deletions)
- **Core files modified** - Critical files like `main.rs`, `Cargo.toml`, etc.
- **New feature** - Commit message indicates feature work

## How to Use

When you complete work and need to commit:

**Step 1: Stage your changes**
```bash
git add <files>
```

**Step 2: Run this skill**
Ask Claude:
```
/smart-commit
```

**Step 3: Claude will:**
1. Run `git status --short` to count files
2. Run `git diff --stat` to count line changes
3. Read `.claude-pr-policy.json` for thresholds
4. Check if any core files are modified
5. Display analysis and decision reasoning
6. Execute either `/pr` or `/commit` based on the decision

## Expected Output

**For significant changes (PR created):**
```
🔍 Analyzing changes...
- Files changed: 12
- Lines changed: 245 additions, 87 deletions (332 total)
- Core files modified: server/launcher/src/main.rs, Cargo.toml

✓ Threshold exceeded: files_changed (12 >= 10)
✓ Threshold exceeded: lines_changed (332 >= 100)
✓ Threshold exceeded: core_files

📋 Changes are SIGNIFICANT. Creating PR for review...

[Executes /pr skill]
```

**For minor changes (direct commit):**
```
🔍 Analyzing changes...
- Files changed: 2
- Lines changed: 15 additions, 3 deletions (18 total)
- Core files modified: none

✓ Changes are minor (< 10 files, < 100 lines, no core files)

Committing to dev...

[Executes /commit skill]
```

## Configuration

Edit `.claude-pr-policy.json` to customize thresholds:

```json
{
  "thresholds": {
    "files_changed": 10,
    "lines_changed": 100,
    "core_files": [
      "server/launcher/src/main.rs",
      "Cargo.toml",
      "package.json"
    ]
  },
  "pr_required_if": "any"
}
```

**Customization options:**
- **Increase thresholds** → More direct commits, fewer PRs
- **Decrease thresholds** → More PRs, more review
- **Add core files** → Always require PR for specific files
- **Change to "all"** → Require ALL conditions (stricter)

## Manual Override

If you want to force a specific workflow:

```
/pr         # Always create PR (ignore thresholds)
/commit     # Always commit directly (ignore thresholds)
/wip        # Quick WIP commit without push
```

## Implementation Steps

When invoked, follow these steps **exactly**:

### 1. Analyze Changes

Run in parallel:
```bash
git status --short
git diff --stat
```

Calculate:
- **files_changed**: Count lines from `git status --short` matching `^\s*[MADRCU]`
- **lines_changed**: Sum additions + deletions from `--stat`

### 2. Load Policy

Read `.claude-pr-policy.json`:
```bash
cat .claude-pr-policy.json
```

Extract:
- `thresholds.files_changed`
- `thresholds.lines_changed`
- `thresholds.core_files[]`
- `pr_required_if` (defaults to "any")

### 3. Check Core Files

For each file in `core_files[]`, check if it appears in `git status --short` output.

### 4. Evaluate Conditions

Check each condition:
- [ ] `files_changed >= threshold`
- [ ] `lines_changed >= threshold`
- [ ] Any core file is modified

### 5. Make Decision

**If `pr_required_if` is "any":**
- If ANY condition is true → Execute `/pr`
- If ALL conditions are false → Execute `/commit`

**If `pr_required_if` is "all":**
- If ALL conditions are true → Execute `/pr`
- Otherwise → Execute `/commit`

### 6. Execute & Report

Before executing:
```
Show user:
- Analysis summary (files, lines, core files)
- Which thresholds were exceeded
- Clear decision with reasoning
```

Then execute the appropriate skill:
- Significant changes → `/pr`
- Minor changes → `/commit`

## Error Handling

- **If `.claude-pr-policy.json` missing**: Use defaults (10 files, 100 lines, empty core files)
- **If `git` commands fail**: Assume PR needed (safe default)
- **If no changes staged**: Inform user and exit
- **If not in git repo**: Error and exit

## Safety Rules

1. **Always show reasoning** - Tell user why you chose PR vs commit
2. **Be conservative** - When in doubt, create a PR
3. **Verify staged changes** - Ensure there's something to commit
4. **Respect existing skills** - Use `/pr` and `/commit` as-is, don't reimplement

## Default Configuration

If `.claude-pr-policy.json` is missing or cannot be read, use these defaults:

```json
{
  "thresholds": {
    "files_changed": 10,
    "lines_changed": 100,
    "core_files": []
  },
  "pr_required_if": "any"
}
```

**What the defaults mean:**
- **files_changed: 10** - Create PR if 10 or more files are modified
- **lines_changed: 100** - Create PR if 100 or more lines changed (additions + deletions)
- **core_files: []** - No core files defined, so this condition never triggers
- **pr_required_if: "any"** - Create PR if ANY threshold is exceeded (OR logic)
