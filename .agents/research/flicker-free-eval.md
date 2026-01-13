# Research: Flicker-Free Evaluation in Zed

**Date:** 2026-01-13
**Status:** Phase 1 COMPLETE, Phase 2 ready
**BD Tasks:**
- ~~zed-supercollider-ysj (Phase 1)~~ DONE
- zed-supercollider-57z (Phase 2: keybindings) READY
- zed-supercollider-8v2 (Phase 3: code lenses)
- zed-supercollider-hai (Phase 4: cleanup)
**Branch:** `feature/lsp-code-action-eval`

---

## Phase 1 Implementation Summary (COMPLETE)

**What was built:**
- Flicker-free eval via LSP code actions (Cmd+. menu)
- Delegated to sclang's existing CodeActionProvider (not custom parsing)
- sclang provides: "SC: Evaluate Line", "SC: Evaluate Block", "SC: Evaluate Selection"

**Key insight:** sclang's LanguageServer.quark already has a CodeActionProvider at:
`server/quark/LanguageServer.quark/Providers/CodeActionProvider.sc`

It uses `LSPDatabase.getDocumentRegions(doc)` for proper block detection. We just forward requests to sclang instead of implementing our own parser.

**What was removed:**
- ~350 lines of naive paren-matching code we initially wrote
- document_cache (no longer needed)
- supercollider.evaluate command (sclang uses .evaluateSelection)

**Commits on branch:**
1. `feat(launcher): add LSP code action for flicker-free eval` - initial naive implementation
2. `feat(launcher): LSP code action eval with block detection` - improved paren matching
3. `refactor(launcher): delegate code actions to sclang` - final elegant solution

---

## How to Resume (Phase 2)

```bash
# 1. Switch to the feature branch
git checkout feature/lsp-code-action-eval

# 2. Check Phase 2 task
bd show zed-supercollider-57z

# 3. Mark as in progress
bd update zed-supercollider-57z --status=in_progress
```

**Phase 2 Goal:** Update keybindings so Cmd+Enter triggers code action eval instead of task

**Key files:**
- `.zed/tasks.json` - project-level keyboard shortcuts
- `languages/SuperCollider/tasks.json` - extension-level task definition
- `languages/SuperCollider/keymap.json` - if we need custom keybindings

**Phase 2 approach options:**
1. Bind Cmd+Enter to `editor::ToggleCodeActions` with filter for SC files
2. Use `workspace::SendKeystrokes` to trigger Cmd+. programmatically
3. Create custom keybinding that calls the code action directly

---

## Problem Statement

When evaluating SuperCollider code in Zed (via play button or Cmd+Enter), a terminal tab flickers open and closed (<100ms). This happens on every eval, creating a disruptive user experience.

### Root Cause

Zed's task system **always creates a terminal tab**, even with these settings:
- `"reveal": "never"` - only controls focus, not tab creation
- `"hide": "always"` - removes tab after completion, causing the flicker
- `"use_new_terminal": false` - reuses terminal, but tab still appears briefly

From [Zed's documentation](https://zed.dev/docs/tasks):
> `reveal: never` — "do not alter focus, but **still add/reuse the task's tab** in its pane"

The fundamental issue: there is no "truly invisible" task mode in Zed.

---

## Options Considered

### Option 1: Accept the Flicker
**Verdict:** Rejected

The flicker is <100ms but happens on every single eval. For a workflow where you might eval dozens of times per minute, this adds up to significant visual noise.

### Option 2: File Zed Feature Request for Invisible Tasks
**Verdict:** Worth doing, but passive

Request a `create_tab: false` or similar option for tasks. Timeline unknown—depends on Zed team priorities.

### Option 3: LSP Code Actions (SELECTED)
**Verdict:** Best immediate solution

Replace task-based eval with LSP code actions that trigger `workspace/executeCommand`. The LSP server handles the eval internally via HTTP—no terminal involvement at all.

**Pros:**
- No terminal, no flicker
- Pure LSP, clean architecture
- Single implementation serves multiple UI entry points

**Cons:**
- Requires LSP changes (moderate effort)
- Code actions appear in Cmd+. menu rather than as gutter play buttons

### Option 4: LSP Code Lenses (SELECTED as complement)
**Verdict:** Good for visual play buttons

Code lenses provide inline "▶ Evaluate" text that triggers `workspace/executeCommand`. Same backend as code actions, different UI.

**Pros:**
- Clickable visual indicators (like current play buttons)
- No terminal involvement

**Cons:**
- Inline text vs gutter icons (different visual style)
- Click-only, not keyboard-accessible

### Option 5: Wait for SendToTerminal (PR #42467)
**Verdict:** Best long-term solution, but uncertain timeline

Zed [PR #42467](https://github.com/zed-industries/zed/pull/42467) adds `editor: send to terminal` action. This would enable sending code directly to sclang running in a visible terminal.

**Pros:**
- Eliminates HTTP/task complexity entirely (~600 lines could be deleted)
- Immediate feedback (see code + result in same terminal)
- Simple mental model
- Matches how many SC users already work

**Cons:**
- Depends on external PR, timeline unknown
- Requires user to have sclang running in terminal
- Changes the workflow model

**Tracking:** BD task zed-supercollider-ow4 monitors this PR monthly.

### Option 6: Named Pipe Workaround
**Verdict:** Rejected - doesn't solve the problem

Idea: Post Window terminal listens on a named pipe, eval task just writes to pipe.

Problem: The task still runs in a terminal, still creates a tab, still flickers. Moving where the work happens doesn't change the task system behavior.

### Option 7: Background the Curl Command
**Verdict:** Rejected - doesn't solve the problem

Idea: Make eval.sh exit immediately by backgrounding the curl.

Problem: Task completion speed doesn't matter—the tab is created when the task starts, not when it finishes.

---

## The Keyboard Eval Challenge

We need two distinct keyboard behaviors (matching SC IDE):
1. **Cmd+Return**: Evaluate enclosing `()` block
2. **Shift+Return**: Evaluate current line only

### Why This Is Hard

- LSP code actions don't know which keybinding triggered them
- If we return two code actions ("Evaluate Block" and "Evaluate Line"), the menu pops up requiring selection
- Zed's `ToggleCodeActions` doesn't support filtering by kind in keybindings

### Research: Does Zed Support Code Action Kind Filtering?

**Finding:** No.

From [Zed source code](https://github.com/zed-industries/zed/blob/main/crates/editor/src/actions.rs):
```rust
pub struct ToggleCodeActions {
    #[serde(skip)]  // Can't set from keybinding!
    pub deployed_from: Option<CodeActionSource>,
    #[serde(skip)]  // Can't set from keybinding!
    pub quick_launch: bool,
}
```

Both fields are `#[serde(skip)]`, meaning they cannot be configured from JSON keybindings.

### Solution: SendKeystrokes Workaround

Use `workspace::SendKeystrokes` to select the line before triggering code actions:

```json
"shift-enter": ["workspace::SendKeystrokes", "cmd-shift-l cmd-."]
```

This:
1. Selects current line (`cmd-shift-l` or equivalent)
2. Triggers code actions (`cmd-.`)

The code action sees a selection and evaluates it—effectively line eval.

**Single code action with smart behavior:**
- Selection exists → evaluate selection
- No selection → find enclosing block → else current line

**Result:**
- `Cmd+Enter` → no selection → block eval
- `Shift+Enter` → line selected → line eval

Both use the same code action, both flicker-free.

**Trade-off:** After Shift+Enter, the line remains selected. This actually provides visual feedback about what was evaluated.

---

## Zed's Code Action Auto-Execute Behavior

Key discovery from [Zed docs](https://zed.dev/docs/tasks):
> "The task will run immediately **if there are no additional Code Actions for this line**."

This means: if "Evaluate" is the **only** code action, Cmd+. executes it immediately without showing a menu.

For SuperCollider files, this should be the case—no other LSP provides code actions for `.scd` files.

---

## Implementation Plan

### Phase 1: Core LSP Implementation
**Task:** zed-supercollider-ysj

1. Add `workspace/executeCommand` handler for `supercollider.evaluate`
   - Arguments: code text
   - Makes HTTP POST to `/eval` endpoint
   - Returns success/failure

2. Add `textDocument/codeAction` handler
   - Returns single "Evaluate" code action
   - Smart code extraction:
     - Selection → use selection
     - No selection → find enclosing `()` block
     - No block → use current line
   - Command triggers executeCommand

**Files:** `server/launcher/src/main.rs`

### Phase 2: Keybindings
**Task:** zed-supercollider-57z

Update `.zed/keymap.json`:
```json
{
  "context": "Editor && (extension == sc || extension == scd)",
  "bindings": {
    "cmd-enter": "editor::ToggleCodeActions",
    "shift-enter": ["workspace::SendKeystrokes", "cmd-shift-l cmd-."]
  }
}
```

Need to verify correct select-line keystroke in Zed.

### Phase 3: Code Lenses
**Task:** zed-supercollider-8v2

Add `textDocument/codeLens` handler for visual play buttons:
- Returns lenses for top-level evaluable regions
- Same executeCommand backend as code actions
- Title: "▶ Evaluate"

### Phase 4: Cleanup
**Task:** zed-supercollider-hai

Remove old task-based eval:
- `tools/eval*.sh` scripts
- Eval tasks from `.zed/tasks.json`
- Potentially `runnables.scm` (if code lenses replace it)
- Update documentation

---

## Architecture Comparison

### Current (Task-Based)
```
User clicks play button
    ↓
runnables.scm matches code_block
    ↓
Zed spawns task (creates terminal tab)
    ↓
tools/eval.sh runs
    ↓
curl POST to http://127.0.0.1:57130/eval
    ↓
Launcher forwards to sclang
    ↓
Task completes (terminal tab removed → FLICKER)
```

### Proposed (LSP Code Action)
```
User hits Cmd+. or clicks code lens
    ↓
Zed requests code actions from LSP
    ↓
LSP returns "Evaluate" action
    ↓
User confirms (or auto-execute if only action)
    ↓
Zed sends workspace/executeCommand
    ↓
LSP makes HTTP POST to /eval internally
    ↓
Done (no terminal involvement)
```

---

## Future: SendToTerminal Integration

When Zed PR #42467 merges, we can offer an alternative workflow:

```
User runs sclang in terminal
    ↓
User hits Cmd+Enter
    ↓
Zed sends code to terminal via SendToTerminal
    ↓
sclang evaluates, output appears in terminal
```

**Benefits:**
- Even simpler (no HTTP layer)
- Immediate visual feedback
- Matches traditional SC workflow

**This would allow deleting:**
- `server/launcher/src/http.rs` (443 lines)
- All `tools/eval*.sh` scripts
- HTTP-related task definitions

The code action approach we're implementing now is a stepping stone—it solves the flicker problem immediately while we wait for SendToTerminal to mature.

---

## Implementation Status

**Phase 1: Core LSP Implementation - COMPLETE (2026-01-13)**

Commit: d3b430a - `feat(launcher): add LSP code action for flicker-free eval`

What was implemented:
1. **Document content cache** - HashMap tracking document content on didOpen/didChange/didClose
2. **textDocument/codeAction handler** - Returns single "Evaluate" action with extracted code
3. **workspace/executeCommand handler** - Sends eval to sclang via UDP, no terminal

Key architecture:
- Direct UDP path: Code Action → executeCommand → launcher → UDP → sclang
- No HTTP involved in the eval path (HTTP server kept for backward compatibility)
- Fire-and-forget: responds to Zed immediately, doesn't wait for sclang

**Remaining Phases:**
- Phase 2: Keybindings (zed-supercollider-57z)
- Phase 3: Code Lenses (zed-supercollider-8v2)
- Phase 4: Cleanup (zed-supercollider-hai)

## Testing Checklist

1. [ ] Open .scd file with LSP running
2. [ ] Cmd+. shows "Evaluate" code action
3. [ ] If only action, executes immediately (no menu)
4. [ ] Code evaluates correctly (check Post Window output)
5. [ ] **NO terminal tab flicker**
6. [ ] Shift+Enter evaluates current line only (Phase 2)
7. [ ] Cmd+Enter evaluates enclosing block (Phase 1 Step 4 - deferred)
8. [ ] Code lens appears and works (Phase 3)
9. [ ] Graceful failure when server not running

## How to Test Phase 1

```bash
# 1. Build the launcher
cd server/launcher && cargo build --release

# 2. Restart Zed (to pick up the new launcher)

# 3. Open a .scd file

# 4. Place cursor on a line like: "hello".postln

# 5. Press Cmd+. to trigger code actions

# 6. You should see "Evaluate" action

# 7. Select it (or auto-execute if only action)

# 8. Check Post Window - should see "hello" printed

# 9. Verify NO terminal tab flicker
```

**Debug mode:** Set `SC_LAUNCHER_DEBUG=1` to see verbose logging in the launcher stderr.

---

## References

- [Zed Tasks Documentation](https://zed.dev/docs/tasks)
- [Zed PR #42467 - SendToTerminal](https://github.com/zed-industries/zed/pull/42467)
- [LSP Code Action Specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_codeAction)
- [LSP Execute Command Specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_executeCommand)
- Related: `.agents/proposals/terminal-eval-ux.md` (detailed UX analysis)
