# Proposal: Terminal-Based Evaluation & UX Improvements

**Status:** Research / Future Consideration
**Date:** 2026-01-13
**Triggered by:** [Zed PR #42467 - Send Code to Terminal](https://github.com/zed-industries/zed/pull/42467)

---

## Executive Summary

Zed PR #42467 introduces `SendToTerminal`, enabling direct code-to-terminal evaluation. Beyond the architectural simplification (eliminating ~850 lines of HTTP workaround code), this presents an opportunity to **significantly improve the user experience** for SuperCollider developers in Zed.

This proposal analyzes current UX pain points, how terminal-based evaluation could address them, and additional UX improvements we could layer on top.

---

## Part 1: Current UX Pain Points

### From `.agents/testing.md` - Known Gaps vs SC IDE

| Feature | SC IDE | Zed Extension | Severity |
|---------|--------|---------------|----------|
| Server status | Always visible in status bar | Must run task to check | **HIGH** |
| CPU/RAM meter | Built-in meter | Not available | **HIGH** |
| Post Window | Docked panel, integrated | `tail -F` log file in terminal | **GAP** |
| Scope/Freqscope | Built-in visualization | External window only | **GAP** |
| Node tree | Built-in panel | Not available | **GAP** |
| Eval feedback | Inline flash/highlight | None - fire and forget | **MEDIUM** |
| Shortcut discovery | Menu bar, tooltips | Must know shortcuts | **MEDIUM** |

### Evaluation UX Issues

**1. Fire-and-Forget Execution**
```
Current flow:
  Click play → HTTP 202 → ... silence ...

User must:
  - Open separate Post Window terminal
  - Watch for output there
  - Hope nothing failed silently
```

**2. Disconnected Feedback Loop**
- Code is in editor panel
- Output is in terminal panel (if open)
- Errors go to log file
- No visual connection between what was evaluated and what happened

**3. Play Button Limitations**
```scheme
; From runnables.scm - only top-level nodes get buttons
(source_file
  (code_block) @code @run ...)
```
- Nested blocks: no play button
- Must use keyboard shortcuts
- Users don't discover this intuitively

**4. Multiple Eval Methods = Confusion**
```
Play button     → tools/eval.sh      → HTTP /eval
Cmd+Return      → SC: Evaluate Block → tools/eval-block.sh → HTTP /eval
Shift+Return    → SC: Evaluate Line  → tools/eval-line.sh  → HTTP /eval
Selection eval  → SC: Evaluate "..." → tools/eval-selection.sh → HTTP /eval
```
Four different paths to the same endpoint, with subtle behavioral differences.

**5. Post Window as Afterthought**
```json
// .zed/tasks.json
{
  "label": "SC: Post Window",
  "command": "tail",
  "args": ["-F", "/tmp/sclang_post.log"]
}
```
- User must manually open
- Not integrated with editor
- Easy to miss errors
- No syntax highlighting

---

## Part 2: How Terminal-Based Eval Improves UX

### The Terminal as Unified Feedback Surface

```
┌─────────────────────────────────────────────────────────────┐
│ Editor                          │ Terminal (sclang REPL)    │
│                                 │                           │
│ // User's code                  │ sc3>                      │
│ (                               │                           │
│   var freq = 440;               │                           │
│   { SinOsc.ar(freq) }.play      │                           │
│ )                               │                           │
│     ↓                           │                           │
│   [SendToTerminal]              │                           │
│     ↓                           │                           │
│                                 │ sc3> (                    │
│                                 │   var freq = 440;         │
│                                 │   { SinOsc.ar(freq) }.play│
│                                 │ )                         │
│                                 │ -> Synth('temp__0': 1000) │
│                                 │                           │
└─────────────────────────────────────────────────────────────┘
```

### UX Improvements

| Pain Point | Current | With Terminal Eval |
|------------|---------|-------------------|
| Feedback delay | Fire-and-forget, check logs | Immediate in same view |
| Output location | Separate log file | Right there in terminal |
| Error visibility | Buried in log | Obvious, inline |
| What was evaluated | No indication | You see the sent code |
| Expression detection | Limited to tagged nodes | Any syntax node |
| Mental model | Complex (HTTP, tasks, scripts) | Simple (send to terminal) |

### Specific Improvements

**1. Immediate Visual Feedback**
- Code appears in terminal as it's sent
- Result appears immediately below
- Errors show in context

**2. Unified Output Stream**
- No separate Post Window needed
- All output in one place
- Easy to scroll back and see history

**3. Better Expression Detection**
PR #42467's approach:
```
Searches upward through syntax ancestors looking for nodes containing
'call', 'statement', 'expression', 'assignment', or 'binary_operator'
```
vs our current explicit node matching in runnables.scm.

**4. Simpler Mental Model**
```
Before: "Click play button, it runs a task, which runs a script,
        which curls an endpoint, which sends UDP, which..."

After:  "Send code to terminal"
```

---

## Part 3: Additional UX Opportunities

Beyond SendToTerminal, we could improve UX in several ways:

### 3.1 Integrated Post Window Panel

**Current:** `tail -F /tmp/sclang_post.log` in a terminal

**Better:** Dedicated Zed panel with:
- Syntax highlighting for SC output
- Clickable error locations (jump to file:line)
- Filter by message type (errors, server, user)
- Clear button
- Auto-scroll toggle

**Implementation:** Would require Zed panel API (not currently available to extensions). Track for future.

### 3.2 Status Bar Integration

**Current:** No server status visibility

**Better:** Status bar showing:
```
SC: ● Server Running | CPU: 12% | Synths: 3
```

**Implementation:** Zed status bar API for extensions. Track as feature request.

### 3.3 Evaluation Highlighting

**Current:** No visual indication of evaluated region

**Better:** Brief highlight/flash on evaluated code:
```
┌─────────────────────────┐
│ (                       │  ← Flash yellow briefly
│   { SinOsc.ar }.play    │  ← when evaluated
│ )                       │
└─────────────────────────┘
```

**Implementation:** Would require editor decoration API. Could potentially use existing Zed highlight mechanisms.

### 3.4 Inline Results (Jupyter-style)

**Current:** Results only in Post Window / terminal

**Better:** Show result inline below code:
```supercollider
{ SinOsc.ar(440) }.play
// → Synth('temp__0' : 1000)
```

**Implementation:** Would require virtual text / inline decoration API. Track for future.

### 3.5 Smart Keybinding Hints

**Current:** Users must know shortcuts

**Better:**
- Hover tooltip on play button: "Cmd+Return to evaluate"
- Context menu shows shortcuts
- First-run welcome with key shortcuts

**Implementation:** Partially possible now via documentation. Full implementation needs Zed UI hooks.

### 3.6 Server Control Panel

**Current:** Separate tasks for boot/stop/recompile

**Better:** Floating panel or sidebar:
```
┌─────────────────────────┐
│ SuperCollider Server    │
│ ● Running               │
│                         │
│ [Boot] [Stop] [Recomp]  │
│                         │
│ CPU: ████░░░░ 45%       │
│ Synths: 12              │
│ Groups: 3               │
└─────────────────────────┘
```

**Implementation:** Would require Zed panel API. Track for future.

---

## Part 4: Terminal Workflow Design

### Recommended Terminal Setup

**Automatic sclang Terminal:**
When user opens a `.scd` file, offer to start sclang in terminal:
```
┌─────────────────────────────────────────────┐
│ SuperCollider file detected.                │
│ Start sclang in terminal? [Yes] [No] [Always]│
└─────────────────────────────────────────────┘
```

Or document the manual workflow:
1. Open terminal panel (`Cmd+J` or `Ctrl+``)
2. Run `sclang`
3. Use `SendToTerminal` to evaluate code

### Keyboard Shortcuts (SC IDE Compatible)

| Action | Shortcut | Behavior |
|--------|----------|----------|
| Evaluate region | Cmd+Return | SendToTerminal with expression detection |
| Evaluate line | Shift+Return | SendToTerminal for current line |
| Stop sounds | Cmd+. | Send `CmdPeriod.run` to terminal |
| Boot server | Cmd+B | Send `Server.default.boot` to terminal |
| Recompile | Cmd+K | Send `thisProcess.recompile` to terminal |

### Boot/Stop/Recompile in Terminal Model

Instead of HTTP endpoints, send commands directly:

```supercollider
// Boot
Server.default.boot

// Stop (CmdPeriod)
CmdPeriod.run

// Recompile (requires sclang restart)
// This is trickier - need to handle in terminal
0.exit  // then user restarts sclang
```

**Challenge:** Recompile requires restarting sclang process. Options:
1. Document manual restart workflow
2. Create a wrapper script that handles restart
3. Keep HTTP endpoint for recompile only

---

## Part 5: Comparison Matrix

### Current vs Terminal-Based UX

| UX Aspect | Current (HTTP) | Terminal-Based | Winner |
|-----------|----------------|----------------|--------|
| Immediate feedback | No | Yes | Terminal |
| See what was sent | No | Yes | Terminal |
| Error visibility | Log file | Inline | Terminal |
| Expression detection | Limited | Flexible | Terminal |
| Setup complexity | Automatic | Manual terminal | HTTP |
| Play buttons | Work | Need rethinking | HTTP |
| Post Window | Separate | Integrated | Terminal |
| Server control | HTTP endpoints | Terminal commands | Tie |
| Recompile handling | HTTP endpoint | Process restart | HTTP |
| Debugging | Multiple logs | One terminal | Terminal |

**Overall:** Terminal-based approach wins on feedback and simplicity, but requires solving the setup and recompile workflows.

---

## Part 6: Implementation Phases

### Phase 1: Research & Validation (No Code Changes)

**Track PR #42467:**
- Monitor for merge
- Note any API changes
- Test when available

**Test Questions:**
1. Does expression detection work for SC syntax?
   - `( )` parenthesized blocks
   - `{ }` function blocks
   - Method chains: `SinOsc.ar(440).dup`
   - Variable definitions

2. Does cursor continuation feel right for SC workflow?

3. Can we send multi-line code blocks?

### Phase 2: Document Alternative Workflow

**Create documentation for terminal-based workflow:**
```markdown
## Alternative: Terminal-Based Evaluation

1. Open terminal panel (Cmd+J)
2. Start sclang: `sclang`
3. Use `editor: send to terminal` to evaluate code

Benefits:
- Immediate feedback
- See input and output together
- No separate post window needed
```

**Keep HTTP as default** - let users choose.

### Phase 3: Improve Current UX (Independent of Terminal)

These improvements work with current architecture:

1. **Better Post Window task:**
   - Syntax highlighting (if possible)
   - Auto-open on first eval

2. **Improve runnables.scm:**
   - Add more expression types
   - Better nested block handling (if grammar allows)

3. **Documentation:**
   - Keyboard shortcut cheat sheet
   - First-run guide
   - Video demo

### Phase 4: Terminal-First Architecture (If Validated)

If Phase 1-2 validate terminal approach:

**Delete:**
- `server/launcher/src/http.rs` (443 lines)
- `tools/eval*.sh` (5 files, ~150 lines)
- `tools/boot-server.sh`, `stop.sh`, `recompile.sh`, `quit-server.sh`
- Eval-related tasks from `.zed/tasks.json`

**Simplify:**
- Launcher becomes LSP-only bridge
- runnables.scm optional (for visual indicators only)

**Add:**
- Documentation for terminal workflow
- Optional: sclang auto-start capability

### Phase 5: Future UX Enhancements (Zed API Dependent)

Track these for when Zed APIs become available:

| Feature | Required API | Priority |
|---------|-------------|----------|
| Status bar integration | Status bar extension API | High |
| Integrated Post Window | Panel extension API | High |
| Eval highlighting | Editor decoration API | Medium |
| Inline results | Virtual text API | Medium |
| Server control panel | Panel extension API | Low |

---

## Part 7: Risk Assessment

### UX Risks of Terminal Approach

| Risk | Impact | Mitigation |
|------|--------|------------|
| Users unfamiliar with terminal | Medium | Clear documentation, optional workflow |
| sclang startup messages noisy | Low | Document filtering, or suppress |
| Recompile workflow different | Medium | Document, or keep HTTP for recompile |
| Two sclang processes confusing | Low | Clear documentation |
| Play buttons lose meaning | Medium | Keep for visual indication only |

### UX Risks of Staying with HTTP

| Risk | Impact | Mitigation |
|------|--------|------------|
| Fire-and-forget frustration | Medium | Better Post Window integration |
| Debugging difficulty | Medium | Better logging, docs |
| Complexity for contributors | Low | Good architecture docs |

---

## Part 8: Success Metrics

How do we know if terminal-based UX is better?

### Qualitative
- User feedback on Discord/GitHub
- Fewer "why didn't my code run?" questions
- Positive comparisons to SC IDE workflow

### Quantitative (If Measurable)
- Time from eval to seeing result
- Number of support questions about eval
- Extension adoption/retention

---

## Part 9: Recommendation

### Immediate Actions

1. **Create beads issue** to track PR #42467
2. **No code changes** until PR merges and stabilizes

### Short-Term (1-2 months)

1. **Test SendToTerminal** with SuperCollider when available
2. **Document terminal workflow** as alternative
3. **Improve current UX** independent of terminal:
   - Better Post Window docs
   - Keyboard shortcut visibility
   - First-run experience

### Medium-Term (3-6 months)

Based on validation results:
- If terminal UX is clearly better → Begin migration
- If mixed results → Support both workflows
- If terminal UX has issues → Stay with HTTP, improve it

### Long-Term (6+ months)

1. **Track Zed extension APIs** for:
   - Status bar integration
   - Panel API for Post Window
   - Editor decorations for eval highlighting

2. **Implement enhancements** as APIs become available

---

## Appendix A: PR #42467 Details

**Status:** Open, under review
**Reviewer:** cameron1024

**Key Implementation:**
- Action: `editor: send to terminal`
- Expression detection via syntax tree traversal
- Terminal integration via `terminal.input()`
- Appends newline (configurable in future)

**Concerns Raised:**
1. Automatic range inference may be controversial
2. Language extensions should customize selection
3. Vim mode integration needs thought
4. Newline behavior should be configurable

**Agreed Direction:**
1. Basic selected-text first
2. Language extension traits for customization
3. Cursor continuation support

---

## Appendix B: Current Architecture Inventory

### Files That Would Be Deleted

| File | Lines | Purpose |
|------|-------|---------|
| `server/launcher/src/http.rs` | 443 | HTTP server |
| `tools/eval.sh` | 24 | Play button handler |
| `tools/eval-line.sh` | 25 | Line eval |
| `tools/eval-block.sh` | 50 | Block eval |
| `tools/eval-selection.sh` | 25 | Selection eval |
| `tools/boot-server.sh` | 10 | Boot via HTTP |
| `tools/stop.sh` | 10 | Stop via HTTP |
| `tools/recompile.sh` | 10 | Recompile via HTTP |
| `tools/quit-server.sh` | 10 | Quit via HTTP |
| **Total** | **~607** | |

### Files That Would Be Simplified

| File | Current | After |
|------|---------|-------|
| `.zed/tasks.json` | 13 tasks | 3-4 tasks |
| `runnables.scm` | Triggers eval | Visual only (optional) |
| `main.rs` | LSP + HTTP threads | LSP only |

---

## Appendix C: SC IDE Feature Parity Checklist

| Feature | SC IDE | Current | With Terminal | Future (API) |
|---------|--------|---------|---------------|--------------|
| Code evaluation | Built-in | HTTP workaround | Native | - |
| Immediate feedback | Yes | No | Yes | - |
| Post Window | Docked panel | Log tail | Terminal | Panel API |
| Server status | Status bar | Task | Terminal output | Status API |
| CPU meter | Built-in | No | No | Status API |
| Eval highlighting | Yes | No | No | Decoration API |
| Help lookup | Menu + shortcut | Shortcut + hover | Same | - |
| Boot/Stop/Recompile | Menu + shortcut | Task shortcuts | Terminal cmds | - |

---

## Appendix D: Questions to Answer

### Before Migration

1. Does SendToTerminal expression detection work for SC?
2. Can multi-line `( )` blocks be sent correctly?
3. How do we handle sclang startup in terminal?
4. What about recompile (requires process restart)?
5. Should play buttons remain for visual indication?

### For Future UX

1. When will Zed have status bar API for extensions?
2. When will Zed have panel API for extensions?
3. Can we get editor decoration support?
4. Is there appetite for inline results (Jupyter-style)?

---

## Conclusion

Terminal-based evaluation via SendToTerminal offers significant UX improvements:
- Immediate, visible feedback
- Unified output stream
- Simpler mental model
- ~600 fewer lines of workaround code

The main tradeoffs are setup complexity (user must start sclang in terminal) and loss of the current "it just works" play button experience.

**Recommendation:** Track PR #42467, test when available, document as alternative workflow. If validation is positive, migrate to terminal-first architecture. Continue tracking Zed extension APIs for additional UX improvements.

**Tracking Issue:** See beads issue for PR monitoring and next steps.
