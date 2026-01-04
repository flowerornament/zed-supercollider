# Usage

Quick start for day-to-day SuperCollider work in Zed.

## Initial Setup

1. Configure the LSP launcher (see [SETTINGS.md](SETTINGS.md))
2. Set up keybindings (see [Keybindings](#keybindings) below)

## Workflow

1. **Open the Post Window** (once per session)
   - Press `ctrl-shift-p` or run "SuperCollider: Post Window" from the command palette
   - This shows all sclang output: server boot messages, errors, `.postln`, eval results
   - Keep this terminal tab visible to monitor SuperCollider

2. **Open a `.scd` file**
   - Language server starts automatically
   - Play buttons appear in the gutter for runnable code blocks

3. **Boot the audio server**
   - Press `cmd-shift-b` or click the play button next to `s.boot`
   - Watch the Post Window for boot messages

4. **Evaluate code**
   - Click play button in gutter, or press `cmd-enter`
   - Results appear in the Post Window
   - No terminal popups - evaluation happens silently in the background

5. **Stop sounds**
   - Press `cmd-.` to stop all audio (CmdPeriod)

## Keybindings

Add these to your Zed keymap (`~/.config/zed/keymap.json`):

```json
[
  {
    "context": "Editor && extension == scd",
    "bindings": {
      "cmd-enter": ["task::Spawn", { "task_name": "SuperCollider: Evaluate" }],
      "cmd-.": ["task::Spawn", { "task_name": "SuperCollider: Stop (CmdPeriod)" }],
      "cmd-shift-b": ["task::Spawn", { "task_name": "SuperCollider: Boot Server" }],
      "cmd-shift-l": ["task::Spawn", { "task_name": "SuperCollider: Recompile" }],
      "ctrl-shift-p": ["task::Spawn", { "task_name": "SuperCollider: Post Window" }],
      "cmd-alt-q": ["task::Spawn", { "task_name": "SuperCollider: Quit Server" }],
      "cmd-alt-k": ["task::Spawn", { "task_name": "SuperCollider: Kill All (Emergency Cleanup)" }]
    }
  }
]
```

### Keybinding Reference

| Shortcut | Action | Description |
|----------|--------|-------------|
| `cmd-enter` | Evaluate | Execute the code block at cursor |
| `cmd-.` | Stop | Stop all sounds (CmdPeriod) |
| `cmd-shift-b` | Boot Server | Start the audio server |
| `cmd-shift-l` | Recompile | Recompile the class library |
| `ctrl-shift-p` | Post Window | Open the post window |
| `cmd-alt-q` | Quit Server | Stop the audio server |
| `cmd-alt-k` | Kill All | Emergency cleanup - kills all SC processes |

## Tips

- **Post Window shows everything**: Server messages, eval results, errors, `.postln` output
- **No distractions**: Evaluating code never steals focus or pops up windows
- **Standard shortcuts**: `cmd-enter` to eval, `cmd-.` to stop (matches SC IDE)
- **Emergency cleanup**: Use "SuperCollider: Kill All" if things get stuck

## Coming from scnvim?

| scnvim | Zed |
|--------|-----|
| `editor.send_line/block/selection` | Click play button or `cmd-enter` |
| Post buffer toggle | `ctrl-shift-p` for Post Window |
| `:SCNvimStart` / `:SCNvimStop` | `cmd-shift-b` / `cmd-alt-q` |
| `:SCNvimRecompile` | `cmd-shift-l` |
| Hard stop | `cmd-.` |
