# ZED_SELECTED_TEXT Workarounds

## Problem

`ZED_SELECTED_TEXT` environment variable is not set when running Zed tasks, even with text selected. This affects all modes (vim and non-vim).

Other variables like `ZED_FILE`, `ZED_ROW`, `ZED_COLUMN` work correctly.

## Solution: Use ZED_ROW/ZED_COLUMN instead

The **"SuperCollider: Help (Cursor)"** task uses `ZED_FILE`, `ZED_ROW`, and `ZED_COLUMN` to extract the word at cursor position.

### Workflow (Recommended)

1. Position cursor on a class name (e.g., `SinOsc`)
2. Run "SuperCollider: Help (Cursor)" task
3. Done - no yank or selection required

This uses `tools/word-at-cursor.sh` to read the file and extract the word at the cursor position.

## Alternative: CodeAction

`cmd+.` on a class name shows "Show Help for {ClassName}" - this uses LSP which receives cursor position directly.

---

## Legacy: Clipboard-based workflow

The `SuperCollider: Help` task (without "Cursor") uses clipboard:

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

## Upstream issue

Filed: https://github.com/zed-industries/zed/issues/46572

Related PRs (for vim selection in other contexts):
- https://github.com/zed-industries/zed/pull/25133 (assistant panel selection)
- https://github.com/zed-industries/zed/pull/29019 (copy and trim)
