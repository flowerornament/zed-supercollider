title: "Testing Guide"
created: 2026-01-12
updated: 2026-01-12
purpose: "Comprehensive testing matrix covering user workflows, agent verification, and UX comparison with SC IDE"
---

# Testing Guide

This document defines all user paths and testing scenarios for the Zed SuperCollider extension. Use it for:
- Manual user acceptance testing
- Agent-assisted verification
- UX comparison with SC IDE
- Regression testing before releases

---

## Quick Test Commands

```bash
# Verify grammar parses test file
cd grammars/supercollider && tree-sitter generate && tree-sitter parse ../../tests/test_runnables.scd

# Test runnables query
tree-sitter query ../../languages/SuperCollider/runnables.scm ../../tests/test_runnables.scd

# Build everything
./scripts/build.sh

# Check running processes
ps aux | grep -E '(sclang|scsynth|sc_launcher)' | grep -v grep

# Check ports
lsof -i :57110 -i :57120 -i :57130

# Emergency cleanup
pkill -f sc_launcher && pkill -f sclang && pkill -f scsynth
```

---

## 1. First Launch Experience

### Setup Verification
| Step | Expected | Pass? |
|------|----------|-------|
| Open any `.scd` file | Syntax highlighting appears | |
| Wait 3-5 seconds | "LSP READY" in launcher output | |
| Hover over class name | Documentation popup | |
| Type `SinOsc.` | Completion menu appears | |

### Failure Recovery
| Scenario | Recovery Action |
|----------|-----------------|
| No highlighting | Reload extensions (Zed > Extensions > Reload) |
| No completion | Check launcher running: `ps aux \| grep sc_launcher` |
| Port conflict | Kill orphaned processes, restart Zed |

---

## 2. Code Evaluation (Core Workflow)

### Play Buttons (Runnables)

| Code Pattern | Button Expected? | Notes |
|--------------|------------------|-------|
| `(1 + 1)` | YES | Top-level grouped expression |
| `(var x = 1; x * 2)` | YES | Multi-statement code block |
| `(\nSynthDef(\\test, {...}).add\n)` | YES | Outer parens only |
| `SinOsc.ar((440))` | NO on inner | Nested paren - false positive bug |
| `{|x| x * 2}` | NO | Function definition, not evaluable |
| `1 + 1;` | NO | Bare statement |
| `~foo = { ... }` | NO | Assignment, not block |

### Evaluation Methods

| Method | Trigger | Scope |
|--------|---------|-------|
| Play button | Click gutter icon | Detected block |
| Eval Selection | Select + Shift+Return | Exactly selected text |
| Eval Line | Cursor + Cmd+Shift+Return | Current line |
| Eval Block | Cursor in parens | Enclosing `()` |

### Verification Flow
1. Create test file with all patterns above
2. Verify button placement matches expectation
3. Click each button, verify code executes
4. Check Post Window for results

---

## 3. Server Control

### Task Execution

| Task | Shortcut | HTTP Endpoint | Expected Result |
|------|----------|---------------|-----------------|
| Boot Server | Cmd+B | `/boot` | scsynth starts |
| Stop (CmdPeriod) | Cmd+. | `/stop` | All sounds stop immediately |
| Recompile | Cmd+K | `/recompile` | Class library reloads |
| Quit Server | Task menu | `/quit` | Graceful shutdown |
| Kill All | Task menu | (pkill) | Emergency cleanup |

### Server State Verification
```supercollider
// Run these to verify server state
s.serverRunning;  // Should return true after boot
s.numSynths;      // Number of active synths
s.avgCPU;         // CPU load
```

---

## 4. LSP Features

### Navigation

| Feature | Test Action | Expected |
|---------|-------------|----------|
| Hover | Hover over `SinOsc` | Shows class doc with methods |
| Goto Definition | Cmd+Click on class | Opens class source file |
| Find References | Right-click > Find References | Lists all uses |
| Document Symbols | Cmd+Shift+O | Shows file outline |
| Workspace Symbols | Cmd+T, type class name | Finds across workspace |

### Completion

| Trigger | Context | Expected Completions |
|---------|---------|----------------------|
| `.` | After object | Instance methods |
| `(` | After class name | Class methods |
| `~` | Start of identifier | Environment variables |
| Partial class name | `Sin` | SinOsc, SinOscFB, etc. |

### Signature Help
1. Type `SinOsc.ar(`
2. Should show: `freq, phase, mul, add`
3. Type `,` to advance parameter highlight

---

## 5. UX Comparison: Zed vs SC IDE

### Feature Parity

| Feature | SC IDE | Zed Extension | Gap? |
|---------|--------|---------------|------|
| Syntax highlighting | Yes | Yes | - |
| Autocomplete | Yes | Yes | - |
| Hover docs | Yes | Yes | - |
| Code evaluation | Cmd+Return | Play button / Shift+Return | Different paradigm |
| Server meter (CPU/RAM) | Visible in GUI | Not visible | **GAP** |
| Scope/Freqscope | Built-in | External window | **GAP** |
| Help browser | Built-in | Browser/terminal | Different |
| Post Window | Docked panel | Tail log file | **GAP** |
| Node tree | Built-in | Not available | **GAP** |
| Recording | Built-in | Manual | **GAP** |

### Discoverability Issues

| Problem | SC IDE | Zed | Severity |
|---------|--------|-----|----------|
| Finding eval shortcut | Menu visible | Must know Shift+Return | Medium |
| Server status | Always visible | Must run task | High |
| CPU load | Meter in corner | Must eval `s.avgCPU` | High |
| Stop sounds | Cmd+. obvious | Cmd+. conflicts initially | Medium |
| Help access | Cmd+D opens browser | Cmd+D task | Low |

### Suggested Improvements

1. **Status Bar Integration**: Show server status, CPU, synth count
2. **Post Window Panel**: Integrated view instead of tail command
3. **Discoverable Shortcuts**: Better documentation or keybinding hints
4. **Visual Feedback**: Flash or highlight when code evaluates

---

## 6. Edge Cases & Error Recovery

### CPU Overload Recovery
1. Run: `100.do { SinOsc.ar(rrand(200, 2000)).play }`
2. System should respond to Cmd+. (CmdPeriod)
3. If unresponsive, Kill All task should work
4. Verify can boot fresh server after

### Process Cleanup
1. Quit Zed while server running
2. Reopen Zed and `.scd` file
3. Launcher should clean up orphaned processes
4. No "port in use" errors

### Recompile During Load
1. Run heavy SynthDef
2. Immediately Cmd+K to recompile
3. Should not crash
4. May lose running sounds (expected)

### Network/Port Issues
1. Start another app on port 57130
2. Extension should show clear error
3. Kill conflicting app
4. Retry should work

---

## 7. Syntax Highlighting Verification

| Element | Example | Expected Color |
|---------|---------|----------------|
| Class name | `SinOsc` | Type (often blue/teal) |
| Method | `.ar` | Function |
| Number | `440` | Number |
| String | `"hello"` | String |
| Symbol | `\freq` | Symbol/constant |
| Comment | `// note` | Comment (muted) |
| Env var | `~buffer` | Variable |
| Keyword | `var`, `arg` | Keyword |
| Operator | `+`, `*` | Operator |

---

## 8. Test Files

### Minimal Smoke Test
```supercollider
// tests/smoke_test.scd
// 1. Should have play button
(1 + 1)

// 2. Server boot (after running above)
s.boot;

// 3. Sound (should play and stop with Cmd+.)
(
{ SinOsc.ar(440, 0, 0.1) }.play;
)

// 4. Hover over SinOsc - should show docs
SinOsc

// 5. Completion - type . after this:
SinOsc
```

### Play Button Edge Cases
```supercollider
// tests/test_runnables.scd

// SHOULD have play buttons:
(1 + 1)
(var x = 1; x)
(
SynthDef(\test, { Out.ar(0, SinOsc.ar) }).add;
)

// Should NOT have play buttons:
SinOsc.ar((440))     // nested
foo((1 + 1))         // nested
{|x| x * 2}          // function def
1 + 1;               // bare statement
```

### Stress Test
```supercollider
// tests/stress_test.scd

// CPU load test
(
50.do { |i|
    { SinOsc.ar(200 + (i * 50), 0, 0.01) }.play;
};
)

// Recovery test - run this then Cmd+.
// Should stop all sounds
```

---

## 9. Agent Testing Checklist

Use this for automated verification:

```
[ ] Grammar parses without errors
    tree-sitter parse tests/smoke_test.scd

[ ] Runnables query matches expected patterns
    tree-sitter query languages/SuperCollider/runnables.scm tests/test_runnables.scd

[ ] Extension builds without errors
    ./scripts/build.sh

[ ] Launcher starts and responds
    curl http://127.0.0.1:57130/health

[ ] Eval endpoint works
    curl -X POST -d "1+1" http://127.0.0.1:57130/eval

[ ] No orphaned processes after tests
    ! pgrep -f "sc_launcher.*orphan"
```

---

## 10. Release Testing Protocol

Before any release:

1. **Clean environment**
   - Remove dev extension
   - Install from scratch
   - Verify first-launch experience

2. **Core workflows**
   - Run smoke test file
   - Verify all play buttons correct
   - Test all keyboard shortcuts

3. **LSP verification**
   - Hover, completion, goto definition
   - Document symbols

4. **Server control**
   - Full boot → play → stop → quit cycle
   - Recovery from CPU overload

5. **Edge cases**
   - Process cleanup after force quit
   - Port conflict recovery
   - Recompile during operation

6. **Documentation**
   - README accurate
   - Keyboard shortcuts documented
   - Known issues listed
