# ZED_SELECTED_TEXT Vim Mode Workarounds

## Problem

`ZED_SELECTED_TEXT` environment variable is empty when Zed's vim mode is enabled, even with text visually selected. Works fine with vim mode disabled.

## Root Cause

Vim mode maintains its own `Mode` enum (Visual, VisualLine, VisualBlock) separately from the editor's selection system. When `task::Spawn` triggers, vim likely exits visual mode before the selection is captured, leaving `ZED_SELECTED_TEXT` empty.

Similar issues were fixed in:
- PR #25133: assistant panel selection
- PR #29019: copy and trim

## Workaround Approaches

### 1. Clipboard Workaround (RECOMMENDED)

Use system clipboard (`pbpaste` on macOS) instead of `ZED_SELECTED_TEXT`.

**Workflow:**
1. Select text in vim visual mode
2. Yank to system clipboard (`"+y` or configure `use_system_clipboard: "on_yank"`)
3. Run task that reads from clipboard

**Task Example:**
```json
{
  "label": "SC Help (Clipboard)",
  "command": "bash",
  "args": ["-c", "SYMBOL=$(pbpaste | tr -d '\\n'); [ -z \"$SYMBOL\" ] && { printf 'Class: '; read SYMBOL; }; HELP=\"/Applications/SuperCollider.app/Contents/Resources/HelpSource/Classes/${SYMBOL}.schelp\"; [ -f \"$HELP\" ] && pandoc -f \"$ZED_WORKTREE_ROOT/tools/schelp/schelp.lua\" -t markdown \"$HELP\" | glow -p || echo \"Not found: $SYMBOL\""],
  "reveal": "always",
  "use_new_terminal": true
}
```

**Settings for vim clipboard:**
```json
{
  "vim": {
    "use_system_clipboard": "on_yank"
  }
}
```

**Keybinding (yank + spawn):**
```json
{
  "context": "Editor && vim_mode == visual",
  "bindings": {
    "space h": ["workspace::SendKeystrokes", "\" + y escape :SC Help (Clipboard) enter"]
  }
}
```

**Pros:** Works reliably with vim mode
**Cons:** Requires extra yank step, overwrites clipboard

---

### 2. Mouse Selection (WORKS)

Mouse selections work even with vim mode enabled.

**Workflow:**
1. Double-click to select word (or click-drag)
2. Run task normally

**Pros:** Simple, no config needed
**Cons:** Requires mouse, breaks keyboard-only workflow

---

### 3. SendKeystrokes Action Chain (UNTESTED)

Chain yank and task spawn in a single keybinding.

**Keybinding:**
```json
{
  "context": "Editor && vim_mode == visual",
  "bindings": {
    "space h": ["workspace::SendKeystrokes", "\" + y"]
  }
}
```

Then bind another key to spawn the clipboard-based task.

**Limitation:** SendKeystrokes can't dispatch async actions like task::Spawn in the same sequence.

---

### 4. Prompt Fallback (CURRENT)

Fall back to prompting user for input when selection is empty.

**Task Example:**
```json
{
  "label": "SC Help",
  "command": "bash",
  "args": ["-c", "SYMBOL=\"$ZED_SELECTED_TEXT\"; [ -z \"$SYMBOL\" ] && { printf 'Class: '; read SYMBOL; }; ..."],
  ...
}
```

**Pros:** Always works
**Cons:** Extra typing required

---

### 5. ZED_SYMBOL Variable (BEST FOR SINGLE WORDS)

Use `ZED_SYMBOL` instead - captures the symbol under cursor via Tree-sitter.

From Zed docs: "ZED_SYMBOL is the currently selected symbol; it should match the last symbol shown in a symbol breadcrumb."

**Task Example:**
```json
{
  "label": "SC Help (Symbol)",
  "command": "bash",
  "args": ["-c", "SYMBOL=\"${ZED_SYMBOL:-}\"; [ -z \"$SYMBOL\" ] && { printf 'Class: '; read SYMBOL; }; HELP=\"/Applications/SuperCollider.app/Contents/Resources/HelpSource/Classes/${SYMBOL}.schelp\"; [ -f \"$HELP\" ] && pandoc -f \"$ZED_WORKTREE_ROOT/tools/schelp/schelp.lua\" -t markdown \"$HELP\" | glow -p || echo \"Not found: $SYMBOL\""],
  "reveal": "always",
  "use_new_terminal": true
}
```

**Pros:** No selection needed, works with vim mode, Tree-sitter powered
**Cons:** Only works for recognized symbols (classes, functions), cursor must be on the symbol

---

## Recommended Setup

For best experience with vim mode, use a cascading fallback:

1. Try clipboard (for yanked text)
2. Try ZED_SYMBOL (for cursor position)
3. Prompt user (last resort)

### Task with Cascading Fallback

```json
{
  "label": "SC Help (Clipboard)",
  "command": "bash",
  "args": ["-c", "SYMBOL=$(pbpaste | head -1 | awk '{print $1}' | sed 's/[.([{].*$//'); [ -z \"$SYMBOL\" ] && SYMBOL=\"${ZED_SYMBOL:-}\"; [ -z \"$SYMBOL\" ] && { printf 'Class: '; read SYMBOL; }; HELP=\"/Applications/SuperCollider.app/Contents/Resources/HelpSource/Classes/${SYMBOL}.schelp\"; [ -f \"$HELP\" ] && pandoc -f \"$ZED_WORKTREE_ROOT/tools/schelp/schelp.lua\" -t markdown \"$HELP\" | glow -p || echo \"Not found: $SYMBOL\""],
  "reveal": "always",
  "use_new_terminal": true
}
```

### Optional: Configure vim clipboard

```json
// settings.json
{
  "vim": {
    "use_system_clipboard": "on_yank"
  }
}
```

### Usage

**Option A - Yank first:**
1. Select text with `viw` or visual mode
2. Yank with `y` (goes to system clipboard with above setting)
3. Run "SC Help (Clipboard)" task

**Option B - Just position cursor:**
1. Position cursor on class name
2. Run "SC Help (Clipboard)" task
3. ZED_SYMBOL provides the symbol under cursor

**Option C - Type it:**
1. Run task with nothing selected/yanked
2. Type class name when prompted

## Testing Notes

| Method | Vim Mode | Works? | Notes |
|--------|----------|--------|-------|
| ZED_SELECTED_TEXT | enabled | NO | Bug - empty |
| ZED_SELECTED_TEXT | disabled | YES | Normal behavior |
| Mouse selection | enabled | YES | Mouse bypasses vim |
| Clipboard (pbpaste) | enabled | YES | After yank |
| ZED_SYMBOL | enabled | YES | Word under cursor |
| Prompt fallback | enabled | YES | Manual input |

## Upstream Issue

This should be reported to zed-industries/zed. Similar fixes:
- https://github.com/zed-industries/zed/pull/25133
- https://github.com/zed-industries/zed/pull/29019
