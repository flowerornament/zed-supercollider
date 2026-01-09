---
description: Merge dev to main after verification
---

Merge verified changes from dev to main.

**When to use:** After confirming dev is stable (builds, tests pass, work is complete).

## Steps

1. Verify dev is ahead of main:
   ```bash
   git fetch origin
   git log --oneline origin/main..origin/dev
   ```
   If empty, nothing to release.

2. Run verification on current code:
   ```bash
   cd server/launcher && cargo check && cargo test
   ```
   If this fails, fix issues first - do NOT proceed.

3. Fast-forward main to dev:
   ```bash
   git push origin origin/dev:main
   ```

4. Verify:
   ```bash
   git fetch origin
   git log --oneline -3 origin/main
   ```

## Rules

- NEVER force push
- NEVER merge if verification fails
- This is a fast-forward only - if main has diverged, resolve manually
- After release, dev and main should be at the same commit
