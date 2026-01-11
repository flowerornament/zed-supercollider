# Help Docs Feature

**Goal**: Cursor on symbol → invoke action → see SuperCollider help documentation

**Status**: REVISED - Using CodeAction approach instead of Hover. See ADR-004.

---

## UPDATE 2026-01-10: CodeAction Approach (Recommended)

The hover integration approach below was implemented and tested, but **rejected** because:
1. Conflates two features: hover (implementation info) vs docs lookup
2. Changes existing behavior users relied on
3. Makes hover responses very long

**New approach**: Use LSP CodeActions
- `cmd+.` on class name → "Show Help for SinOsc" → opens docs in new tab
- See: `.ai/decisions/004-codeaction-for-help.md`
- Implementation: beads task `zed-supercollider-0mr`

**What's already built and working**:
- `/convert-schelp` endpoint in launcher
- `LSPDatabase.findSchelpPath()` and `fetchSchelpMarkdown()` helpers
- Just need to wire up via CodeAction instead of Hover

---

## HISTORICAL: Hover Approach (Rejected)

---

## Architecture

```
User: cursor on "SinOsc" → presses K (or mouse hovers)
                ↓
Zed: sends textDocument/hover LSP request
                ↓
Quark HoverProvider: extracts "SinOsc" from document
                ↓
Quark: finds /path/to/SinOsc.schelp
                ↓
Launcher: POST /convert-schelp → pandoc → markdown
                ↓
Quark: returns hover response with markdown
                ↓
Zed: displays formatted documentation popup
```

**Why this works**: LSP hover is triggered by both mouse hover AND keyboard (`K` in vim mode). No selection/clipboard issues.

---

## What's Already Built

| Component | Status | Location |
|-----------|--------|----------|
| HoverProvider | Working | `server/quark/.../Providers/HoverProvider.sc` |
| schelp→markdown converter | Complete | `tools/schelp/schelp.lua` (pandoc reader) |
| HTTP server | Working | `server/launcher/src/http.rs` |
| Test fixtures | Complete | `tools/schelp/test/*.schelp` |

**Current hover response**: Returns symbol name + first `/* */` comment from .sc file (minimal).

**What we're adding**: Full schelp documentation converted to markdown.

---

## Implementation Plan

### Step 1: Add `/convert-schelp` endpoint to launcher

**File**: `server/launcher/src/http.rs`

```rust
// POST /convert-schelp
// Body: {"path": "/path/to/SinOsc.schelp"}
// Returns: {"markdown": "# SinOsc\n\n..."}

fn handle_convert_schelp(request: &mut Request) -> Response {
    // 1. Parse JSON body for "path"
    // 2. Verify file exists
    // 3. Run: pandoc -f tools/schelp/schelp.lua -t markdown {path}
    // 4. Return markdown in JSON response
}
```

**Add route** in `handle_http_request()`:
```rust
if url == "/convert-schelp" && method == Method::Post {
    return handle_convert_schelp(request);
}
```

### Step 2: Enhance HoverProvider to fetch schelp docs

**File**: `server/quark/.../Providers/HoverProvider.sc`

After getting `wordAtCursor` and resolving to class:

```supercollider
// Find schelp file
var schelpPath = this.findSchelpPath(cls);
var schelpMarkdown;

schelpPath !? {
    // Call launcher endpoint
    schelpMarkdown = this.fetchSchelpMarkdown(schelpPath);
};

// Add to contents
schelpMarkdown !? {
    contents = contents.add((
        language: "markdown",
        value: schelpMarkdown
    ));
};
```

### Step 3: Add helper methods to HoverProvider or LSPDatabase

```supercollider
findSchelpPath { |cls|
    // SuperCollider help is at:
    // /Applications/SuperCollider.app/Contents/Resources/HelpSource/Classes/{ClassName}.schelp
    var helpDir = Platform.resourceDir +/+ "HelpSource/Classes";
    var path = helpDir +/+ cls.name.asString ++ ".schelp";
    if (File.exists(path)) { ^path };
    ^nil
}

fetchSchelpMarkdown { |path|
    // HTTP POST to launcher
    var url = "http://127.0.0.1:57130/convert-schelp";
    // Use NetAddr or shell out to curl
    // Return markdown string
}
```

### Step 4: Add caching (optional optimization)

Cache converted markdown by file path + mtime to avoid repeated conversions.

---

## Testing

1. **Test converter manually**:
   ```bash
   pandoc -f tools/schelp/schelp.lua -t markdown \
     /Applications/SuperCollider.app/Contents/Resources/HelpSource/Classes/SinOsc.schelp
   ```

2. **Test endpoint** (after implementing):
   ```bash
   curl -X POST http://127.0.0.1:57130/convert-schelp \
     -H "Content-Type: application/json" \
     -d '{"path": "/Applications/SuperCollider.app/Contents/Resources/HelpSource/Classes/SinOsc.schelp"}'
   ```

3. **Test in Zed**:
   - Open a .scd file
   - Position cursor on `SinOsc`
   - Press `K` (vim mode) or hover with mouse
   - Should see full documentation

---

## Dependencies

- `pandoc` must be installed on user's machine
- Launcher must be running (already required for LSP)
- SuperCollider.app must be installed (for .schelp files)

---

## Files to Modify

1. `server/launcher/src/http.rs` - Add `/convert-schelp` endpoint
2. `server/quark/.../Providers/HoverProvider.sc` - Fetch and include schelp docs
3. `server/quark/.../LSPDatabase.sc` - Add helper for schelp path lookup (optional)

---

## Historical Context

Previous approaches tried before settling on hover integration:

| Approach | Why it failed |
|----------|---------------|
| ZED_SELECTED_TEXT in tasks | Vim mode bug in Zed (selection lost on task spawn) |
| ZED_SYMBOL | Returns scope name, not word under cursor |
| Clipboard-based tasks | Requires explicit yank, poor UX |

Hover integration bypasses all these issues because LSP hover works regardless of vim mode or selection state.

---

## Related Issues

- **zed-supercollider-52i** (P1) - This implementation
- **zed-supercollider-l6b** (P3) - Upstream Zed vim bug (won't fix, hover solves it)
