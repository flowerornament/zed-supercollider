# ADR-001: HTTP Evaluation Instead of LSP executeCommand

**Date:** 2026-01-04
**Status:** Accepted
**Deciders:** Architecture exploration

## Context

Need code evaluation triggered from editor. Standard LSP provides `workspace/executeCommand` for custom commands.

## Problem

Zed extension API cannot programmatically invoke LSP commands:
- No `workspace/executeCommand` support in extension API
- Issue [#13756](https://github.com/zed-industries/zed/issues/13756) open since 2024
- Code Actions can provide menu items but cannot be triggered programmatically
- Extension cannot send LSP requests on behalf of user

## Decision Drivers

1. Must support one-click evaluation (play button in gutter)
2. Must work with current Zed API (v0.x)
3. Should be debuggable independently of editor
4. Should work cross-platform
5. Should support keybindings

## Options Considered

### Option 1: LSP Code Actions
**Approach:** Provider returns Code Action → user selects from menu → executeCommand

**Pros:**
- Standard LSP approach
- No extra infrastructure needed

**Cons:**
- ❌ Requires user to click menu (not one-click)
- ❌ Cannot be triggered by keybinding
- ❌ Extension cannot invoke programmatically
- ❌ Poor UX for live coding workflow

**Result:** Rejected - UX unacceptable

### Option 2: Custom LSP Requests
**Approach:** Extension sends custom LSP request directly

**Pros:**
- LSP-based

**Cons:**
- ❌ Same API limitation - extension cannot send LSP requests
- ❌ Not possible with current Zed extension API
- ❌ Would require Zed core changes

**Result:** Rejected - not technically feasible

### Option 3: HTTP Server in Launcher + Runnables
**Approach:** Launcher runs HTTP server, Tasks POST to it, Runnables provide play buttons

**Pros:**
- ✅ Works with current Zed API (Tasks can run shell commands)
- ✅ One-click via play buttons (runnables + tasks)
- ✅ Debuggable with curl independently
- ✅ Cross-platform (HTTP is universal)
- ✅ Can bind to keybindings (task spawn)
- ✅ Testable without editor

**Cons:**
- Non-standard (not pure LSP)
- Extra port to manage (mitigated: configurable, default 57130)
- Two communication channels to maintain

**Result:** ✅ Accepted

## Decision

Use HTTP server in launcher with Runnables for play buttons.

**Architecture:**
```
User clicks play button
  → runnables.scm captures code block as $ZED_CUSTOM_code
  → Task with tag "sc-eval" executes
  → curl POST http://127.0.0.1:57130/eval --data "$ZED_CUSTOM_code"
  → Launcher HTTP handler receives code
  → Launcher sends workspace/executeCommand via UDP to sclang
  → sclang evaluates, posts result to /tmp/sclang_post.log
  → Post Window task (tail -f) displays result
```

## Consequences

### Positive
- One-click evaluation works (primary requirement met)
- Independently testable with curl
- Works around Zed API limitation
- Future-proof: if Zed adds executeCommand, can migrate or keep both
- Users can test endpoints without Zed

### Negative
- Non-standard approach needs documentation
- Port management (mitigated: configurable port, default non-conflicting)
- Two communication channels (LSP + HTTP) to maintain
- HTTP server adds ~200 LOC to launcher

### Neutral
- Launcher must run both HTTP and LSP servers (acceptable complexity)
- Requires tasks.json configuration (but so would keybindings anyway)

## Validation

Tested and working (2026-01-04):
- ✅ Play buttons appear in gutter for code blocks
- ✅ Clicking play button executes code
- ✅ `curl` testing works independently
- ✅ Tasks execute correctly
- ✅ Keybindings work via `task::Spawn`
- ✅ Results appear in Post Window

Evidence:
```bash
curl -X POST -d "1 + 1" http://127.0.0.1:57130/eval
# Returns: {"result":"2"}

grep -i "sc-eval" .zed/tasks.json
# Shows task matching runnables tag
```

## Implementation Notes

**HTTP Endpoints:**
- `POST /eval` - Execute code (body = source code)
- `POST /stop` - CmdPeriod (hard stop all synths)
- `POST /boot` - Boot audio server
- `POST /recompile` - Recompile class library
- `POST /quit` - Quit audio server
- `GET /health` - Health check

**Runnables:**
- Detects `(code_block)` and `(function_block)` nodes
- Tags with `sc-eval`
- Captures full block text as `$ZED_CUSTOM_code`

**Task Integration:**
```json
{
  "label": "SuperCollider: Evaluate",
  "command": "sh",
  "args": ["-c", "curl -s -X POST --data-binary \"$ZED_CUSTOM_code\" http://127.0.0.1:57130/eval"],
  "tags": ["sc-eval"]
}
```

## Future Considerations

If Zed adds workspace/executeCommand support:
- Can migrate to standard LSP approach
- HTTP server can remain for backward compatibility
- Or deprecate HTTP and remove in major version
- Runnables approach still provides better UX than code actions

## Related Decisions

- See ADR-002 for config field issues
- See ADR-003 for 3-layer architecture rationale

## References

- [Zed Issue #13756](https://github.com/zed-industries/zed/issues/13756) - workspace/executeCommand not available
- [Zed Tasks Documentation](https://zed.dev/docs/tasks)
- [Zed Runnables Blog](https://zed.dev/blog/zed-decoded-tasks)
- [LSP executeCommand Spec](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_executeCommand)
