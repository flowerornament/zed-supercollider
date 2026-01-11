---
title: "ADR-004: CodeAction for Help Docs Instead of Hover"
created: 2026-01-10
updated: 2026-01-10
status: Accepted
purpose: "Architectural decision to use LSP CodeActions for help docs lookup instead of integrating into hover"
---

# ADR-004: CodeAction for Help Docs Instead of Hover

## Context

We want to show SuperCollider help documentation (from .schelp files) when the user's cursor is on a class name. The goal is: cursor on `SinOsc` → invoke action → see full documentation.

## Problem

Several approaches were considered and rejected:

### Attempted: Modify HoverProvider
Initially implemented schelp integration directly in HoverProvider. This:
- Conflated two distinct features: implementation info (original hover) vs full docs
- Changed existing behavior users relied on
- Made hover responses very long for classes with extensive docs

### Rejected: Zed Tasks with Clipboard
Existing Help tasks use clipboard (`pbpaste`):
- Requires `yiw` (yank) before running task - clunky UX
- `ZED_SELECTED_TEXT` doesn't work in vim mode (selection lost on task spawn)
- `ZED_SYMBOL` returns scope name, not word under cursor

### Rejected: Direct Keybinding to LSP Command
Zed cannot pass cursor position when invoking LSP commands via keybinding (see ADR-001).

## Decision

Use LSP CodeActions for help docs lookup.

**Why this works:**
- CodeActions receive document URI and cursor position from the client
- Can be triggered via `cmd+.` (code action menu)
- Potentially bindable to custom keybinding
- Doesn't interfere with hover behavior
- Only shows "Show Help" action when cursor is on a valid class name

**Implementation:**
1. CodeActionProvider detects class name at cursor position
2. Returns "SuperCollider: Show Help for {ClassName}" action
3. Action executes `supercollider.showHelp` command
4. Command converts schelp to markdown and opens in Zed

## Consequences

**Benefits:**
- Separate, dedicated UX for help lookup
- Hover maintains original behavior (implementation info)
- Works in vim mode (no selection required)
- Position-aware (uses LSP cursor position)
- Discoverable via `cmd+.` menu

**Tradeoffs:**
- Requires two keypresses (`cmd+.` then select) instead of one
- Need to verify if Zed supports binding directly to specific code action
- Help action shows in menu even for classes without schelp (error on invoke)

**Implementation Files:**
- `server/quark/.../Providers/CodeActionProvider.sc` - add help action
- `server/quark/.../Providers/ExecuteCommandProvider.sc` - add showHelp command
- Uses existing: `LSPDatabase.findSchelpPath()`, `LSPDatabase.fetchSchelpMarkdown()`, `/convert-schelp` endpoint

## Key Learning

**Don't modify existing LSP providers for new features.** Hover has a specific purpose (quick info on mouseover/K). Adding full documentation conflates features. Instead, find the right LSP mechanism for the use case:
- Quick info → Hover
- Actions at cursor → CodeAction
- Navigation → Definition/References
- Full documentation → CodeAction that opens new document

## References

- Plan file: `/Users/morgan/.claude/plans/synchronous-moseying-cook.md`
- Research: `.ai/research/help-docs-feature.md`
- Related: ADR-001 (HTTP vs LSP for evaluation)
