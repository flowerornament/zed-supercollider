# Migration (scnvim → Zed)

This guide maps common scnvim workflows to Zed.

- Evaluate
  - scnvim: `editor.send_line/block/selection`
  - Zed: SuperCollider: Eval Line / Selection / Block (LSP executeCommand)
- Post Window
  - scnvim: post buffer toggles/clear
  - Zed: language server output panel; optional terminal task (see TASKS_SNIPPET.md)
- Server lifecycle
  - scnvim: start/recompile/hard_stop
  - Zed: Boot / Recompile / Hard Stop / Quit (LSP commands)
- Help
  - scnvim: plain-text or external converter
  - Zed: hover + Open Help buffer; optional external converter path
- Snippets
  - scnvim: generator
  - Zed: bundled JSON snippets

See `docs/reference/scnvim/` for pinned upstream docs and `docs/MIGRATION_PLAN.md` for the full implementation plan.
