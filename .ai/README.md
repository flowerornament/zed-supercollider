# AI Agent Documentation

This directory contains documentation optimized for AI agents (Claude Code, Cursor, etc.)

## Start Here

**Read `context.md` first** - it's the primary entry point with:
- Project overview and current state
- What works, what's broken, what's next
- Critical anti-patterns to avoid
- Quick file map and build commands
- Links to detailed documentation

## Directory Structure

```
.ai/
  README.md                    # This file
  architecture.md              # System design deep dive
  conventions.md               # Code patterns and anti-patterns
  commands.md                  # All build/test/debug commands

  decisions/                   # Architectural Decision Records (ADRs)
    001-http-not-lsp.md        # Why HTTP instead of LSP executeCommand
    002-config-fields.md       # Why minimal config.toml

  research/                    # Investigation findings (dated)
    2026-01-05-navigation.md   # Navigation fix investigation

  prompts/                     # Reusable task templates
    debug-lsp-issue.md         # How to diagnose LSP problems
```

## Other Important Files

- **`.ai/context.md`** - PRIMARY CONTEXT (start here)
- **`/IMPROVEMENTS.md`** - 28+ prioritized enhancement ideas

## What's in Each File

### context.md (7KB)
- Current state and next tasks
- Critical anti-patterns with evidence
- Quick reference for common operations
- Links to all other docs

### architecture.md (18KB)
- System diagram and data flows
- Component responsibilities
- Design decisions and rationale
- Extension points for adding features
- Performance characteristics
- Failure modes and debugging

### conventions.md (14KB)
- SuperCollider code patterns
- Zed extension patterns
- Launcher patterns
- Testing patterns
- Anti-patterns with evidence of why they fail
- File organization guidelines

### commands.md (11KB)
- Build commands (extension vs launcher)
- Test commands
- Debug commands
- Log monitoring
- Verification procedures
- Troubleshooting steps

### decisions/ (ADRs)
Architectural Decision Records explaining:
- What problem was being solved
- What options were considered
- Why we chose what we chose
- What the consequences are
- Evidence that it works

**Critical for avoiding re-litigating past decisions.**

### research/ (Investigation Logs)
Dated research documents showing:
- What problem was investigated
- How the solution was found
- Evidence before/after
- Learnings from the process

**Useful for understanding why things are the way they are.**

### prompts/ (Task Templates)
Step-by-step guides for common tasks:
- Debugging specific issues
- Adding new features
- Testing procedures

**Follow these when you need to perform similar tasks.**

## Usage Principles

1. **Always read context.md first** - it has the current context
2. **Check anti-patterns before coding** - avoid repeating mistakes
3. **Reference ADRs when making architectural decisions** - understand constraints
4. **Use commands.md for verification** - concrete evidence features work
5. **Create new research docs for investigations** - document findings for future
6. **Keep docs fresh as you go** - update `context.md` when state changes, add ADRs for decisions, add research logs for investigations
7. **Git hygiene** - check `git status` before/after, don’t revert user changes, avoid destructive commands, don’t amend existing commits unless asked; keep changes focused to the task

## What Was Removed

Old docs/ directory contained user-facing documentation:
- USAGE.md - User workflow tutorials
- SETTINGS.md - User configuration guides
- TROUBLESHOOTING.md - User troubleshooting
- AGENTS.md, CONTRIBUTING.md - Developer guides
- LOG.md - Historical timeline
- PLAN.md - Implementation plan with completed milestones

**Why removed:** AI agents need technical context and current state, not user tutorials or historical timelines. All essential information migrated to .ai/ structure.

## Adding New Documentation

**Research findings:**
- Create `.ai/research/YYYY-MM-DD-topic.md`
- Include problem, investigation process, solution, evidence

**New architectural decisions:**
- Create `.ai/decisions/NNN-topic.md`
- Use ADR format (context, options, decision, consequences)

**New task templates:**
- Create `.ai/prompts/task-name.md`
- Include step-by-step procedure with commands

**Update context.md:**
- When current state changes
- When new critical anti-patterns discovered
- When major features added/removed
