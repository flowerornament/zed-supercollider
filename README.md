# SuperCollider Zed Extension

Zed extension for SuperCollider with LSP support and HTTP-based code evaluation.

## For AI Agents

**Start here:** [`.ai/context.md`](.ai/context.md) - Primary context with current state, anti-patterns, and quick reference.

**Full documentation:** [`.ai/`](.ai/) directory

## Quick Links

- **Current Status:** See `.ai/context.md` 
- **Architecture:** See `.ai/architecture.md`
- **Commands:** See `.ai/commands.md`
- **Improvements:** See `IMPROVEMENTS.md`

## Project Structure

```
.ai/                   # AI agent documentation (start here)
  context.md          # PRIMARY - current state, anti-patterns, quick ref
  architecture.md     # System design and data flows
  conventions.md      # Code patterns and anti-patterns
  commands.md         # Build/test/debug commands
  decisions/          # Architectural Decision Records
  research/           # Investigation findings
  prompts/            # Task templates

src/lib.rs            # Extension entry point (WASM)
server/launcher/      # LSP bridge + HTTP server (Rust)
server/quark/         # LanguageServer.quark (vendored)
languages/            # Tree-sitter queries, config
IMPROVEMENTS.md       # Enhancement backlog
```

## Current State

**Working:** Navigation (go-to-def, references), evaluation (play buttons), server control
**Next:** Verify navigation after recent provider fix
**See:** `.ai/context.md` for details
