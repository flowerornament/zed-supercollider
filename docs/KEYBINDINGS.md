# Keybindings

Add these keybindings to your Zed keymap (`~/.config/zed/keymap.json`) for quick evaluation and server control.

## Recommended Keybindings

These mirror the standard SuperCollider IDE shortcuts:

```json
[
  {
    "context": "Editor && extension == scd",
    "bindings": {
      "cmd-enter": ["task::Spawn", { "task_name": "SuperCollider: Evaluate" }],
      "cmd-.": ["task::Spawn", { "task_name": "SuperCollider: Stop (CmdPeriod)" }],
      "cmd-shift-b": ["task::Spawn", { "task_name": "SuperCollider: Boot Server" }],
      "cmd-shift-l": ["task::Spawn", { "task_name": "SuperCollider: Recompile" }],
      "cmd-shift-q": ["task::Spawn", { "task_name": "SuperCollider: Quit Server" }]
    }
  }
]
```

## Keybinding Reference

| Shortcut | Action | Description |
|----------|--------|-------------|
| `cmd-enter` | Evaluate | Execute the code block at cursor |
| `cmd-.` | Stop | Stop all sounds (CmdPeriod) |
| `cmd-shift-b` | Boot Server | Start the audio server |
| `cmd-shift-l` | Recompile | Recompile the class library |
| `cmd-shift-q` | Quit Server | Stop the audio server |

## Setup

1. Open Zed's keymap: `cmd-shift-p` → "zed: open keymap"
2. Add the bindings above to your keymap array
3. Save the file - keybindings take effect immediately

## Notes

- The `context` filter ensures these bindings only activate in `.scd` files
- `cmd-.` is the traditional SuperCollider "panic" key to stop all sounds
- If you prefer `ctrl` over `cmd`, substitute accordingly
