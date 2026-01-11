# Help Docs Feature - Research & Status

**Goal**: Cursor over symbol → run command → see help docs rendered nicely in Zed

**Status**: Blocked on Zed's vim mode breaking selection capture. Workarounds exist but have poor UX.

---

## The Core Problem

We want: select/hover on `SinOsc` → see its help documentation.

**Blocker**: Zed's vim mode doesn't populate `ZED_SELECTED_TEXT` for tasks. Visual mode selections are lost when task spawns.

**Why this matters**: Most SC developers use vim keybindings. Without vim support, the feature is effectively broken for the primary user base.

---

## What's Been Built

### 1. schelp→Markdown Converter (COMPLETE)
- **Location**: `tools/schelp/schelp.lua`
- **How it works**: Pandoc Lua reader that parses `.schelp` format → Markdown
- **Tested on**: SinOsc, Pan2, Osc, Demand (see `tools/schelp/test/`)
- **Dependencies**: `pandoc` (must be installed)

### 2. Zed Tasks (PARTIAL)
- **Location**: `.zed/tasks.json`
- **Tasks available**:
  - `SuperCollider: Help` - Clipboard → pandoc+glow/less in terminal
  - `SuperCollider: Help (Browser)` - Clipboard → opens online docs
  - `SuperCollider: Help (Preview)` - Clipboard → markdown → opens in Zed
- **Problem**: All rely on clipboard (pbpaste), requiring explicit yank first

### 3. Documentation
- `docs/vim-selection-workarounds.md` - Documents the yank-first workflow

---

## Approaches Tried

### Approach A: ZED_SELECTED_TEXT
**Result**: FAILED (vim mode)
- Works perfectly with vim mode disabled
- Completely broken with vim mode enabled
- Upstream Zed bug: vim exits visual mode before selection captured
- Related PRs: zed-industries/zed#25133, #29019

### Approach B: ZED_SYMBOL
**Result**: FAILED (wrong data)
- Contains the *scope* cursor is inside (e.g., function name)
- NOT the word under cursor
- Useless for help lookup

### Approach C: Clipboard (pbpaste)
**Result**: WORKS but poor UX
- Requires explicit yank (`yiw`) before running task
- Not discoverable - users don't know to yank first
- Current implementation in tasks.json

### Approach D: Online Docs in Browser
**Result**: WORKS but leaves Zed
- Opens https://docs.supercollider.online/Classes/{Symbol}
- Context switch is disruptive
- Requires internet

### Approach E: Terminal Rendering (glow/less)
**Result**: WORKS but suboptimal
- Renders markdown in terminal panel
- Not as nice as native markdown preview
- Terminal interaction can be awkward

### Approach F: Markdown Preview in Zed
**Result**: WORKS but workflow is clunky
- Converts schelp → /tmp/{Symbol}.md → opens in Zed
- Works well once file is open
- Problem is getting the symbol name in the first place

---

## The Right Solution: Hover Integration (Issue 52i)

Instead of fighting with task selection, integrate help into the LSP hover response.

**How it would work**:
1. User hovers over `SinOsc`
2. LSP hover request goes to quark
3. Quark finds `/path/to/SinOsc.schelp`
4. Launcher converts schelp→markdown (via pandoc or native)
5. Hover response includes formatted documentation

**Why this is better**:
- No selection/clipboard issues
- Works automatically (no task to invoke)
- Native Zed hover UI
- Works identically in vim and normal mode

**Blockers for this approach**:
1. Need `/api/convert-schelp` endpoint in launcher (or call pandoc)
2. Need to modify quark's HoverProvider to find .schelp files
3. Need to handle caching (don't re-convert on every hover)

**Tracked in**: zed-supercollider-52i

---

## Alternative: Word-Under-Cursor via LSP

Could add a custom LSP request to get word under cursor:
1. Add `supercollider/wordAtPosition` request to quark
2. Task calls launcher HTTP endpoint to get word
3. Then proceeds with help lookup

**Pros**: Works around vim selection bug
**Cons**: Round-trip latency, complex for simple feature

---

## Open Issues

| Issue | Priority | Status | Description |
|-------|----------|--------|-------------|
| zed-supercollider-52i | P2 | open | Hover integration (the real fix) |
| zed-supercollider-l6b | P3 | open | Upstream vim bug tracking |
| zed-supercollider-2tj | P3 | in_progress | Original prototype (can close) |

---

## Recommended Next Steps

1. **Close 2tj** - Prototype work is done, learnings captured here
2. **Prioritize 52i** - This is the actual solution
3. **For 52i implementation**:
   - Add HTTP endpoint to launcher: `POST /convert-schelp` with path, returns markdown
   - Modify quark HoverProvider to call conversion for .schelp files
   - Cache converted markdown (file mtime based)
4. **Keep l6b open** - Track upstream, maybe file Zed issue

---

## Quick Reference

**Test the converter manually**:
```bash
pandoc -f tools/schelp/schelp.lua -t markdown \
  /Applications/SuperCollider.app/Contents/Resources/HelpSource/Classes/SinOsc.schelp
```

**Current workaround for users**:
1. Position cursor on class name
2. Yank with `yiw`
3. Run "SuperCollider: Help" task

**Vim clipboard setting** (add to Zed settings):
```json
{
  "vim": {
    "use_system_clipboard": "on_yank"
  }
}
```
