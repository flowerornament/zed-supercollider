# Debug Session: Flicker-Free Eval Code Action Not Appearing

**Date:** 2026-01-13
**Branch:** `feature/lsp-code-action-eval`
**Status:** Debugging - code action not appearing in Cmd+. menu

## Problem

The LSP code action "⚡ Evaluate (no flicker)" is NOT appearing in the Cmd+. menu. User only sees the old task-based "SC: Evaluate" from runnables.scm.

## What Was Implemented

1. **Document cache** - HashMap tracking content on didOpen/didChange/didClose
2. **textDocument/codeAction handler** - Returns "Evaluate" action with code
3. **workspace/executeCommand handler** - Sends eval to sclang via UDP

## Current Debug State

Added verbose logging to codeAction handler (commit 9ad61c0):
- Logs every codeAction request with URI
- Logs cache lookup result
- Logs cache keys if document not found

## Next Steps for Debugging

1. **Restart Zed and check stderr output** - The launcher now logs to stderr
   - Look for `[sc_launcher] codeAction request for...`
   - Check if document cache is being populated

2. **Possible issues to investigate:**
   - Is `textDocument/didOpen` being received and processed?
   - Is the document URI format matching between didOpen and codeAction?
   - Is the codeAction handler even being called?

3. **If code action appears but still flickers:**
   - Zed might not be sending `workspace/executeCommand` to LSP
   - The `command` field might be interpreted as a Zed action, not LSP command

## Key Files

- `server/launcher/src/main.rs` - LSP handlers (lines 1549-1684 for codeAction)
- `.agents/research/flicker-free-eval.md` - Full research doc
- `.zed/tasks.json` - Old task definitions (may conflict)
- `languages/SuperCollider/runnables.scm` - Runnable definitions (may conflict)

## Prompt for Next Session

```
Continue debugging the flicker-free eval feature on branch `feature/lsp-code-action-eval`.

The issue: Our LSP code action "⚡ Evaluate (no flicker)" isn't appearing in Cmd+. menu.

1. First, ask user to restart Zed, open a .scd file, press Cmd+., and share what they see in the terminal/stderr output from the launcher. Look for "[sc_launcher] codeAction request" messages.

2. Based on the output, debug:
   - If no codeAction log: The handler isn't being called - check if LSP is running
   - If "cache lookup NOT FOUND": Document isn't being cached - check didOpen handling
   - If "cache lookup found" but no action visible: Response may not be reaching Zed

3. Key insight: Runnables (from runnables.scm) also appear as "code actions" in Cmd+. menu. Our LSP code action should appear alongside them, not replace them.

4. Read .agents/research/flicker-free-eval.md for full context.
```

## BD Task

zed-supercollider-ysj - Phase 1: Core LSP implementation (IN PROGRESS)
