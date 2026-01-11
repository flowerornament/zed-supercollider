# ZED_SELECTED_TEXT Vim Mode Workarounds

> **Note**: This documents workarounds for task-based help. The preferred solution is LSP hover - cursor on symbol → press `K` → see docs. Once hover integration is complete (issue 52i), these workarounds won't be needed.

## Problem

`ZED_SELECTED_TEXT` environment variable is empty when Zed's vim mode is enabled, even with text visually selected. This is a Zed bug where vim exits visual mode before the selection is captured during task spawn.

## Why ZED_SYMBOL doesn't help

`ZED_SYMBOL` contains the **scope** your cursor is inside (e.g., the function name if you're inside a function), not the word under the cursor. For looking up help on a class name, this is useless.

## How the Help tasks work

The `SuperCollider: Help` tasks use clipboard with prompt fallback:

1. **Clipboard** (`pbpaste`) - Text you've yanked
2. **Prompt** - Manual input if clipboard is empty

## Workflow

1. Position cursor on a class name (e.g., `SinOsc`)
2. Yank with `yiw` (yank inner word) - this copies to system clipboard
3. Run "SuperCollider: Help" task

**Tip:** Configure vim to always use system clipboard:

```json
{
  "vim": {
    "use_system_clipboard": "on_yank"
  }
}
```

## Alternative: Manual input

If you don't want to yank:
1. Run "SuperCollider: Help" task
2. Type the class name when prompted

## Upstream issue

This should be reported to zed-industries/zed. Similar fixes:
- https://github.com/zed-industries/zed/pull/25133 (assistant panel selection)
- https://github.com/zed-industries/zed/pull/29019 (copy and trim)
