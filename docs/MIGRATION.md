# Migration (scnvim → Zed)

This guide maps common scnvim workflows to Zed.

- Evaluate
  - scnvim: `editor.send_line/block/selection`
  - Zed: click the gutter play button for tagged blocks (runnables) or bind a task to a key to POST `$ZED_CUSTOM_code` to the launcher (`/eval`).
- Post Window
  - scnvim: post buffer toggles/clear
  - Zed: task/launcher output in the terminal panel; optional persistent `sclang` task (see `docs/TASKS_SNIPPET.md`)
- Server lifecycle
  - scnvim: start/recompile/hard_stop
  - Zed: tasks POST to `/boot`, `/stop`, `/recompile`, `/quit` on the launcher
- Help
  - scnvim: plain-text or external converter
  - Zed: hover docs via LSP; external SC help browser as needed
- Snippets
  - scnvim: generator
  - Zed: bundled JSON snippets

See `docs/reference/scnvim/` for pinned upstream docs and `PLAN.md` for the current implementation plan.
