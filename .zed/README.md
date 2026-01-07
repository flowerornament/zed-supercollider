# Zed Tasks for SuperCollider

This repository ships a `.zed/tasks.json` that drives the SuperCollider runnables:
- Play buttons use the `sc-eval` tag and route code to the HTTP `/eval` endpoint.
- Control helpers (Post Window, Stop, Boot, Recompile, Quit, Kill) are defined here too.

To use these tasks in another workspace, copy or merge this file into that project's `.zed/tasks.json`. Keep any other `.zed` files (like personal settings) local to your machine.

## Keybindings
- The extension publishes default bindings via `keymaps/supercollider.json` and `extension.toml`. When the extension is installed, Zed loads these automatically for SuperCollider files:
  - `ctrl-alt-shift-enter` Evaluate; `ctrl-alt-shift-m` Manual Evaluate; `ctrl-alt-shift-b` Boot; `ctrl-alt-shift-.` Stop; `ctrl-alt-shift-r` Recompile; `ctrl-alt-shift-q` Quit; `ctrl-alt-shift-k` Kill All; `ctrl-alt-shift-p` Post Window; `ctrl-alt-shift-c` Check Setup.
- For a buffer-style Post window (like Zed's LSP log view), use the `supercollider.internal.openPostLog` command. Default binding: `ctrl-alt-shift-o`. You can also run it via the command palette (`LSP: Execute Command`) if you prefer not to bind it.
- If you want to customize further, copy that block into your personal keymap (`~/.config/zed/keymap.json` on macOS/Linux, `%APPDATA%\\Zed\\keymap.json` on Windows) and adjust as needed.
