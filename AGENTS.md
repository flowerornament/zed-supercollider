# Agent Instructions

This project uses **bd** (beads) for issue tracking. Run `bd prime` for workflow context.

## Project Overview

Ship a stable Zed extension for SuperCollider with navigation, completion, hover, and play-button evaluation. Architecture is dual-channel: LSP over stdio↔UDP for intelligence, HTTP for eval/control (Zed extensions cannot call `workspace/executeCommand`).

**Current state (2026-01-07):**
- Working: go-to-definition, hover, completion, eval/control endpoints
- Partial: references (built-ins can hit fallback issues)
- Known: logging defaults noisy, "Non Boolean in test" crash in references provider

## Documentation Map

- `.agents/architecture.md` - System diagram and mental model
- `.agents/conventions.md` - Code rules for SC/Rust/Zed
- `.agents/commands.md` - Build, verify, and troubleshoot commands
- `.agents/decisions/` - ADRs for architecture choices
- `.agents/prompts/` - Debug checklists
- `.agents/research/` - Past investigations

## Anti-patterns (do not regress)

- `languages/SuperCollider/config.toml`: keep only documented fields. Never add `opt_into_language_servers` or `scope_opt_in_language_servers`.
- SuperCollider dictionary functions: never use `^` (non-local return). Use expression returns.
- Dev launcher: only use local binary when it exists; otherwise honor settings/PATH.
- Vendored quark: edit the copy in repo; avoid overwriting user-installed quark.

## Key Files

- `src/lib.rs` – extension entry, launcher selection
- `server/launcher/src/main.rs` – LSP bridge + HTTP eval/control
- `server/quark/LanguageServer.quark/` – LSP providers (submodule)
- `languages/SuperCollider/config.toml` – language config (stay minimal)
- `.zed/tasks.json` – tasks that hit HTTP endpoints

**Submodule note:** The quark is a git submodule. When working in it, return to the parent directory for `bd` commands (beads lives in parent repo).

## Permissions Note

Some scripts need to launch `/Applications/SuperCollider.app/Contents/MacOS/sclang`. In sandboxed runs this can fail. If that happens, rerun with escalated permissions.

---

## Issue Tracking

**Quick reference:**
```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --status in_progress  # Claim work
bd close <id>         # Complete work
bd sync               # Sync with git
```

## Beads Sync Policy

Beads commits go to the `beads-sync` branch automatically (not main).

**Rules:**
- Never checkout `beads-sync` to do work - it's only for beads state
- The daemon auto-commits beads changes to this branch
- Push beads-sync when pushing code:
  ```bash
  git push                      # Push main/feature branch
  git push origin beads-sync    # Push beads state
  ```

**If beads-sync diverges or gets stale:**
```bash
bd sync  # Syncs beads state to/from beads-sync branch
```

## Git Workflow: Dev Branch

**CRITICAL: Never ship broken builds to main.** Main must always be buildable and functional. All work goes through dev first.

**Why dev?** Multiple Claude Code instances may work in parallel on the same directory. Pushing to `dev` prevents incomplete work from landing on `main`.

**Branches:**
- `main` - Stable, verified code only. **Must always build and work.**
- `dev` - Working branch, receives all `/commit` pushes
- `beads-sync` - Issue tracking state (auto-managed)

**How it works:**
- All Claude Code instances work on `main` locally (no branch switching)
- `/commit` pushes to `origin/dev` (not main)
- `/release` merges verified dev to main **only after verification**

**Before releasing to main:**
1. Build succeeds: `cargo build --target wasm32-wasip2 --release`
2. Extension loads in Zed (test with "zed: reload extensions")
3. Core features work (LSP starts, completion/hover functional)

## Session Completion

**When ending a work session**, complete ALL steps. Work is NOT complete until changes are pushed.

1. **File issues** for remaining work
2. **Run quality gates** (if code changed) - tests, linters, builds
3. **Update issue status** - close finished, update in-progress
4. **COMMIT CHANGES**:
   ```bash
   /commit          # Pushes to dev branch
   ```

   For significant changes that need PR review:
   ```bash
   /pr              # Creates PR to main
   ```

5. **Push beads state**:
   ```bash
   git push origin beads-sync
   ```

6. **Verify completion**:
   ```bash
   git log --oneline origin/dev -3  # Confirm your commits are on dev
   ```

7. **Hand off** - provide context for next session

**Rules:**
- Work is NOT complete until pushed to remote
- `/commit` goes to dev, `/pr` goes to main
- Use `/release` to merge verified dev to main
