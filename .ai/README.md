# AI Agent Documentation

Documentation optimized for AI agents.

## Start Here

Read `context.md` first - primary entry point with project overview, current state, anti-patterns, and quick reference.

## Directory Structure

```
.ai/
  README.md                    # This file
  context.md                   # PRIMARY - current state and quick reference
  architecture.md              # System design deep dive
  conventions.md               # Code patterns and anti-patterns
  commands.md                  # All build/test/debug commands
  tasks/2026-01-05-execution-plan.md # Consolidated backlog/plan

  decisions/                   # Architectural Decision Records (ADRs)
    001-http-not-lsp.md        # Why HTTP instead of LSP executeCommand
    002-config-fields.md       # Why minimal config.toml

  research/                    # Investigation findings (dated)
    2026-01-05-navigation.md   # Navigation fix investigation

  prompts/                     # Reusable task templates
    debug-lsp-issue.md         # How to diagnose LSP problems
```

## What's in Each File

**context.md** - Current state, anti-patterns, quick reference

**architecture.md** - System design, data flows, component responsibilities

**conventions.md** - Code patterns and anti-patterns

**commands.md** - Build, test, debug commands

**tasks/2026-01-05-execution-plan.md** - Consolidated backlog/plan

**decisions/** - Architectural Decision Records (ADRs) documenting key decisions and their rationale

**research/** - Investigation logs showing how problems were solved

**prompts/** - Step-by-step guides for common tasks

## Usage Principles

1. Read context.md first
2. Check anti-patterns before coding
3. Reference ADRs when making architectural decisions
4. Use commands.md for verification
5. Document investigations in research/
6. Keep docs fresh as you work
7. Follow git hygiene

## Adding New Documentation

**Research findings:** Create `.ai/research/YYYY-MM-DD-topic.md` with problem, solution, evidence

**Architectural decisions:** Create `.ai/decisions/NNN-topic.md` using ADR format

**Task templates:** Create `.ai/prompts/task-name.md` with step-by-step procedure

**Update context.md** when state changes or new anti-patterns discovered
