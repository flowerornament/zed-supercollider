# Agent Instructions

This project uses **bd** (beads) for issue tracking. Run `bd prime` for workflow context.

## Project Overview

Ship a stable Zed extension for SuperCollider with navigation, completion, hover, and play-button evaluation. Architecture is dual-channel: LSP over stdio↔UDP for intelligence, HTTP for eval/control (Zed extensions cannot call `workspace/executeCommand`).

## Mind Map

Read `.agents/MIND_MAP.md` first — it's the codebase knowledge index with cross-referenced nodes. Update it when you make significant changes.

## Documentation Map

- `.agents/MIND_MAP.md` - Knowledge graph index (start here)
- `.agents/architecture.md` - System diagram and mental model
- `.agents/conventions.md` - Code rules for SC/Rust/Zed
- `.agents/commands.md` - Build, verify, and troubleshoot commands
- `.agents/decisions/` - ADRs for architecture choices
- `server/launcher/README.md` - Launcher usage and quark discovery
- `server/launcher/http-api.md` - HTTP endpoint details

## Building & Development

Use `just` for all build and development tasks:

```bash
just              # List all commands
just build        # Full release build
just build-debug  # Debug build
just check        # Quality checks (fmt, lint, test)
just fmt          # Format code
just lint         # Run clippy
just lint-strict  # Clippy with warnings as errors
just test         # Run tests
just clean        # Clean artifacts
```

**Requirements:** `just`, `emscripten` (for grammar wasm), `tree-sitter-cli`

**Build steps:**
1. Compiles tree-sitter grammar to `grammars/supercollider.wasm`
2. Builds the launcher binary
3. Builds the Zed extension wasm
4. Runs tests

After building, reinstall the dev extension in Zed and restart.

## Grammar Development

The tree-sitter grammar is in `grammars/supercollider/` (a git submodule).

**Key insight:** Editing `grammar.js` alone is NOT enough. You must:
1. Run `tree-sitter generate` to regenerate `src/parser.c`
2. Run `tree-sitter build --wasm` to compile to `grammars/supercollider.wasm`
3. Reinstall the dev extension in Zed

The build script handles this, but requires **emscripten** (`brew install emscripten`).

**Extension.toml grammar config:**
- `repository` + `rev` → Zed fetches from remote repo (reliable)
- `path` → Zed compiles from local source (problematic with pre-compiled wasm)

**Grammar fork:** We maintain a fork at `github.com/flowerornament/tree-sitter-supercollider` with:
- `grouped_expression` vs `code_block` distinction (prevents nested paren play buttons)
- Upstream: `github.com/madskjeldgaard/tree-sitter-supercollider`

**To deploy grammar changes:**
1. Edit `grammars/supercollider/grammar.js`
2. Run `tree-sitter generate` to regenerate parser
3. Test with `tree-sitter parse`
4. Commit and push to fork: `cd grammars/supercollider && git push fork HEAD:main`
5. Update `extension.toml` rev to new commit SHA

**To merge upstream changes:**
```bash
cd grammars/supercollider
git fetch origin                    # fetch upstream
git checkout origin/main            # checkout latest upstream
git cherry-pick <our-commit> --no-commit  # apply our changes
# resolve conflicts if any
tree-sitter generate                # regenerate parser
git commit && git push fork HEAD:main --force
# update extension.toml rev
```

**Testing grammar locally:**
```bash
cd grammars/supercollider
tree-sitter generate
tree-sitter parse ../../tests/test_file.scd  # Test parsing
tree-sitter query ../../languages/SuperCollider/runnables.scm ../../tests/test_file.scd  # Test queries
```

**Runnables (play buttons):**
- Defined in `languages/SuperCollider/runnables.scm`
- Must match node types from the grammar (`code_block`, `grouped_expression`)
- `@run` marks where button appears, `@code` captures text for `$ZED_CUSTOM_code` (case-sensitive!)
- `(#set! tag sc-eval)` links to task in `languages/SuperCollider/tasks.json`

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
- `languages/SuperCollider/tasks.json` – extension-level tasks (for runnables)
- `.zed/tasks.json` – project-level tasks (keyboard shortcuts)

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
1. Build succeeds: `just build`
2. Extension loads in Zed (test with "zed: reload extensions")
3. Core features work (LSP starts, completion/hover functional)

## Session Completion

**When ending a work session**, complete ALL steps. Work is NOT complete until changes are pushed.

1. **File issues** for remaining work
2. **Run quality gates** (if code changed) - `just check`
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
