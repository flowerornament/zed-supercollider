# SuperCollider Zed Extension

Zed extension for SuperCollider with LSP support and HTTP-based code evaluation.

## For AI Agents

Start here: [`.ai/context.md`](.ai/context.md) - Current state, anti-patterns, quick reference

Full documentation: [`.ai/`](.ai/) directory

## Project Structure

```
.ai/                   # AI agent documentation (start here)
  context.md          # PRIMARY - current state, anti-patterns, quick ref
  architecture.md     # System design and data flows
  conventions.md      # Code patterns and anti-patterns
  commands.md         # Build/test/debug commands
  improvements.md     # Enhancement backlog (prioritized)
  decisions/          # Architectural Decision Records
  research/           # Investigation findings
  prompts/            # Task templates

src/lib.rs            # Extension entry point (WASM)
server/launcher/      # LSP bridge + HTTP server (Rust)
server/quark/         # LanguageServer.quark (vendored)
languages/            # Tree-sitter queries, config
```

## Current State

Working: Navigation, code evaluation, server control

See `.ai/context.md` for details
