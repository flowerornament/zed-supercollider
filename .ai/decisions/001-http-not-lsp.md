# ADR-001: HTTP Evaluation Instead of LSP executeCommand

Date: 2026-01-04
Status: Accepted

## Context

SuperCollider requires code evaluation triggered from the editor. Standard LSP provides `workspace/executeCommand` for custom commands.

## Problem

Zed extension API cannot programmatically invoke LSP commands. No `workspace/executeCommand` support ([Issue #13756](https://github.com/zed-industries/zed/issues/13756)). Code Actions require manual menu selection and cannot be triggered programmatically or via keybindings.

## Options Considered

### LSP Code Actions
User selects from menu to trigger executeCommand. Rejected: requires manual menu selection, cannot be triggered programmatically or via keybinding.

### Custom LSP Requests
Extension sends custom LSP request directly. Rejected: not technically feasible with current Zed API.

### HTTP Server + Runnables
Launcher runs HTTP server, Tasks POST to it, Runnables provide play buttons. Accepted: works with current Zed API, one-click evaluation, independently testable.

## Decision

Use HTTP server in launcher with Runnables for play buttons.

Flow: User clicks play button → runnables.scm captures code → Task POSTs to HTTP endpoint → Launcher forwards to sclang via UDP → Result displayed in Post Window

## Consequences

**Benefits:**
- One-click evaluation via play buttons
- Independently testable: `curl -X POST -d "1 + 1" http://127.0.0.1:57130/eval`
- Works around Zed API limitation
- Future-proof if Zed adds executeCommand support

**Tradeoffs:**
- Non-standard approach (dual-channel architecture)
- Port management (configurable, default 57130)
- Two communication channels (LSP + HTTP)

**Implementation:**
HTTP endpoints: /eval, /stop, /boot, /recompile, /quit, /health. Tasks POST to endpoints, runnables (via runnables.scm) provide play buttons. See `.ai/architecture.md` for details.

**Validation:**
Play buttons appear in gutter, clicking executes code, curl testing works independently, keybindings work via task::Spawn.

## References

- [Zed Issue #13756](https://github.com/zed-industries/zed/issues/13756) - workspace/executeCommand limitation
- See `.ai/architecture.md` for dual-channel architecture details
