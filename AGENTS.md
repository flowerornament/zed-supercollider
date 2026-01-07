# Agent Instructions

This project uses **bd** (beads) for issue tracking. Run `bd prime` for workflow context.

## Project Overview

Ship a stable Zed extension for SuperCollider with navigation, completion, hover, and play-button evaluation. Architecture is dual-channel: LSP over stdio↔UDP for intelligence, HTTP for eval/control (Zed extensions cannot call `workspace/executeCommand`).

**Current state (2026-01-07):**
- Working: go-to-definition, hover, completion, eval/control endpoints
- Partial: references (built-ins can hit fallback issues)
- Known: logging defaults noisy, "Non Boolean in test" crash in references provider

## Documentation Map

- `.ai/architecture.md` - System diagram and mental model
- `.ai/conventions.md` - Code rules for SC/Rust/Zed
- `.ai/commands.md` - Build, verify, and troubleshoot commands
- `.ai/decisions/` - ADRs for architecture choices
- `.ai/prompts/` - Debug checklists
- `.ai/research/` - Past investigations

## Anti-patterns (do not regress)

- `languages/SuperCollider/config.toml`: keep only documented fields. Never add `opt_into_language_servers` or `scope_opt_in_language_servers`.
- SuperCollider dictionary functions: never use `^` (non-local return). Use expression returns.
- Dev launcher: only use local binary when it exists; otherwise honor settings/PATH.
- Vendored quark: edit the copy in repo; avoid overwriting user-installed quark.

## Key Files

- `src/lib.rs` – extension entry, launcher selection
- `server/launcher/src/main.rs` – LSP bridge + HTTP eval/control
- `server/quark/LanguageServer.quark/` – LSP providers
- `languages/SuperCollider/config.toml` – language config (stay minimal)
- `.zed/tasks.json` – tasks that hit HTTP endpoints

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

## Session Completion

**When ending a work session**, complete ALL steps. Work is NOT complete until `git push` succeeds.

1. **File issues** for remaining work
2. **Run quality gates** (if code changed) - tests, linters, builds
3. **Update issue status** - close finished, update in-progress
4. **PUSH TO REMOTE**:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Hand off** - provide context for next session

**Rules:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing
- If push fails, resolve and retry
