---
title: "Architecture"
created: 2026-01-05
updated: 2026-01-05
purpose: "System design, component interactions, data flows, and extension points for the SuperCollider Zed extension"
---

# Architecture

## System Overview

3-layer bridge: **Zed Extension (WASM) ↔ Launcher (Rust) ↔ sclang (UDP)**

## System Diagram

```
┌──────────────────────────────────────────────────────────────┐
│ Zed Editor                                                   │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  runnables.scm → Play Buttons → Tasks → HTTP POST           │
│                                           │                  │
│  LSP Client ─────────────── stdio ────────┼────────┐         │
│                                           │        │         │
└───────────────────────────────────────────┼────────┼─────────┘
                                            │        │
                                            ▼        ▼
┌──────────────────────────────────────────────────────────────┐
│ sc_launcher (Rust)                                           │
│  ┌──────────────────┐  ┌────────────────────────────────┐   │
│  │ HTTP Server      │  │ LSP Bridge                     │   │
│  │ :57130           │  │ (stdio ↔ UDP translation)      │   │
│  │ /eval            │  │                                │   │
│  │ /stop /boot      │  │ Message buffering until ready  │   │
│  │ /recompile /quit │  │ UDP chunking (>8KB)            │   │
│  └────────┬─────────┘  └───────────┬────────────────────┘   │
│           │                        │                         │
│           └────────────┬───────────┘                         │
│                        ▼                                     │
│                   UDP Socket (localhost)                     │
└────────────────────────┼────────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────────┐
│ sclang --daemon                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ LanguageServer.quark                                   │ │
│  │  - TextDocumentProvider (pending queue for early msgs) │ │
│  │  - GotoDefinitionProvider                              │ │
│  │  - ReferencesProvider                                  │ │
│  │  - CompletionProvider                                  │ │
│  │  - ExecuteCommandProvider (supercollider.eval)         │ │
│  │  - LSPDatabase (class/method indexing)                 │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  → stdout/stderr → /tmp/sclang_post.log                     │
└──────────────────────────────────────────────────────────────┘
```

## Dual-Channel Design

### Why Two Channels?

**Standard LSP:** Everything through workspace/executeCommand
**Problem:** Zed extension API cannot invoke workspace/executeCommand
**Solution:** Split into two channels

### Channel 1: LSP (Intelligence)
**Flow:** Zed ↔ stdio ↔ Launcher ↔ UDP ↔ sclang/LanguageServer.quark

**Features:**
- Completions
- Go-to-definition
- Find references
- Document symbols
- Signature help
- Code lens

**Why it works:** Zed's built-in LSP client handles these automatically.

### Channel 2: HTTP (Evaluation & Control)
**Flow:** Zed Tasks → HTTP POST → Launcher → UDP → sclang

**Endpoints:**
- `POST /eval` → Execute code
- `POST /stop` → CmdPeriod (hard stop)
- `POST /boot` → Boot audio server
- `POST /recompile` → Recompile class library
- `POST /quit` → Quit audio server
- `GET /health` → Health check

**Why HTTP:**
- Zed Tasks can run curl
- Debuggable independently
- Cross-platform
- Works around API limitation

## Data Flows

### Flow 1: Go-to-Definition (LSP Channel)

```
1. User cmd-clicks on "SinOsc"
2. Zed sends: {"method":"textDocument/definition","params":{"position":...}}
3. Launcher receives via stdin
4. Launcher forwards via UDP to sclang:57120
5. sclang/GotoDefinitionProvider searches LSPDatabase
6. Returns: {"result":{"uri":"file://...","range":...}}
7. Launcher forwards via stdout
8. Zed navigates to location
```

**Logs to check:**
- Request: `/tmp/sc_launcher_stdin.log`
- Response: stdout (shown in Zed LSP logs)
- Errors: `/tmp/sclang_post.log`

### Flow 2: Code Evaluation (HTTP Channel)

```
1. User clicks play button or presses cmd-enter
2. runnables.scm captures (code_block) → $ZED_CUSTOM_code
3. Task executes: curl POST http://127.0.0.1:57130/eval --data "$ZED_CUSTOM_code"
4. Launcher HTTP server receives
5. Launcher sends: {"method":"workspace/executeCommand","params":{"command":"supercollider.eval","arguments":[code]}}
6. sclang/ExecuteCommandProvider interprets code
7. Result posted to stdout → /tmp/sclang_post.log
8. Post Window task (tail -f) shows result
```

**Logs to check:**
- HTTP request: launcher stdout
- Eval result: `/tmp/sclang_post.log`
- Errors: `/tmp/sclang_post.log`

## Component Details

### Extension (src/lib.rs)

**Responsibilities:**
- Implement `zed::Extension` trait
- Return launcher command and args
- Provide workspace configuration
- Dev mode detection (looks for Cargo.toml)

**Key functions:**
- `language_server_command()` - Returns launcher path
- `language_server_workspace_configuration()` - Sends settings to quark
- Dev mode: `if worktree.read_text_file("Cargo.toml").is_ok()` → use local build

**Does NOT:**
- Parse LSP messages (passes through)
- Handle evaluation (HTTP server does this)

### Launcher (server/launcher/src/main.rs)

**Responsibilities:**
- Translate stdio ↔ UDP for LSP
- Run HTTP server for evaluation
- Spawn and manage sclang process
- Buffer messages until sclang ready
- Chunk large UDP messages

**Key sections:**
- Args/config parsing
- LSP bridge (stdio → UDP, UDP → stdout)
- Message buffering (waits for "***LSP READY***")
- HTTP server (eval and control endpoints)
- Process management (spawn sclang, handle shutdown)

**Critical details:**
- Buffers messages until `***LSP READY***` marker received
- Chunks messages >8KB (UDP limit)
- Forwards stderr to stdout for logging
- Graceful shutdown on stdin close

### Quark (server/quark/LanguageServer.quark/)

**Architecture:** Provider pattern

**Key classes:**
- `LSP` - Main entry point, registers providers
- `LSPConnection` - UDP socket management
- `LSPDatabase` - Index classes/methods for lookups
- `TextDocumentProvider` - Document state, pending queue
- `GotoDefinitionProvider` - Find definitions
- `ReferencesProvider` - Find references
- `CompletionProvider` - Completions
- `ExecuteCommandProvider` - Custom commands (eval)

**Critical details:**
- TextDocumentProvider queues didOpen/didChange until initialized
- LSPDatabase builds index on startup
- ExecuteCommandProvider: NO `^` returns (use if/else expressions)

**Initialization sequence:**
1. LSP.start spawns UDP listener
2. Waits for initialize request
3. Registers all providers
4. Sends "***LSP READY***" marker
5. TextDocumentProvider.processPending() replays queued messages

### Runnables (languages/SuperCollider/runnables.scm)

**Tree-sitter query:**
```scheme
((code_block) @code @run (#set! tag sc-eval))
((function_block) @code @run (#set! tag sc-eval))
```

**How it works:**
- `@code` - Captured as `$ZED_CUSTOM_code` environment variable
- `@run` - Where play button appears in gutter
- `#set! tag sc-eval` - Matches tasks with `"tags": ["sc-eval"]`

**Task matching:**
```json
{
  "label": "SuperCollider: Evaluate",
  "command": "sh",
  "args": ["-c", "curl ... --data-binary \"$ZED_CUSTOM_code\" ..."],
  "tags": ["sc-eval"]
}
```

## Design Decisions

See `.ai/decisions/` for full ADRs:

**001-http-not-lsp.md:** Why HTTP instead of LSP executeCommand
- Zed API limitation (Issue #13756)
- Tasks + HTTP works today
- Provides play buttons and keybindings

**002-config-fields.md:** Why minimal config.toml
- `opt_into_language_servers` breaks navigation
- Extension configs ≠ built-in language configs
- Only use documented Zed fields

**003 (implied):** Why 3-layer architecture
- sclang speaks LSP over UDP (not stdio)
- Zed extensions are WASM (cannot open sockets)
- Launcher bridges the gap

## Extension Points

### Adding LSP Capabilities

1. **Check if Quark supports it:**
   - Look in `LanguageServer.quark/Providers/`
   - Check if provider exists

2. **Implement provider if needed:**
   - Create new provider class
   - Register in `LSP.sc`

3. **Advertise capability:**
   - Add to launcher `initialize` response
   - No extension code change needed

4. **Verify:**
   ```bash
   grep -i "newCapability" /tmp/sc_launcher_stdin.log
   ```

### Adding HTTP Endpoints

1. **Add launcher handler:**
   - Edit `server/launcher/src/main.rs` HTTP section
   - Add route and handler function

2. **Add SuperCollider command:**
   - Edit `ExecuteCommandProvider.sc`
   - Add new command to dictionary

3. **Add task:**
   - Edit `.zed/tasks.json`
   - Add task that calls endpoint

4. **Test:**
   ```bash
   curl -X POST http://127.0.0.1:57130/newEndpoint
   ```

### Adding Runnables

1. **Identify tree-sitter node:**
   - Use Zed's tree-sitter inspector
   - Find node type for code region

2. **Add to runnables.scm:**
   ```scheme
   ((node_type) @code @run (#set! tag sc-newaction))
   ```

3. **Create matching task:**
   ```json
   {
     "tags": ["sc-newaction"],
     "command": "..."
   }
   ```

4. **Test:** Play button should appear

## Performance Characteristics

**Startup time:** ~2-3 seconds
- sclang launch: ~1-2s
- Quark initialization: ~1s
- LSPDatabase indexing: ~0.5s

**Message latency:**
- LSP request → response: <100ms typical
- HTTP eval: ~50ms + execution time

**Bottlenecks:**
- LSPDatabase linear search (no indexing)
- UDP message size (8KB limit, requires chunking)
- sclang interpretation speed

## Failure Modes

### LSP Not Starting
**Symptoms:** No completions, no navigation
**Check:** `tail /tmp/sc_launcher_stdin.log` - should see messages
**Causes:** Launcher not found, sclang not in PATH

### Evaluation Not Working
**Symptoms:** Play button does nothing
**Check:** `curl -X POST -d "1+1" http://127.0.0.1:57130/eval`
**Causes:** HTTP server not running, port conflict

### Navigation Not Working
**Symptoms:** Cmd-click does nothing
**Check:** `grep -i "definition" /tmp/sc_launcher_stdin.log`
**Causes:** Usually config fields (see .ai/decisions/002)

### Quark Crashes
**Symptoms:** Errors in `/tmp/sclang_post.log`
**Check:** Look for "ERROR", "EXCEPTION", "doesNotUnderstand"
**Causes:** Uninitialized vars, nil references, bad returns
