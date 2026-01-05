# Code Conventions & Patterns

## SuperCollider Patterns

### ✅ Safe Dictionary Access
```supercollider
// GOOD: Handle nil keys
allMethodsByName[method.name] = (allMethodsByName[method.name] ?? { Array.new }).add(method);

// BAD: Assumes key exists
allMethodsByName[method.name].add(method);  // ❌ Throws DNU if key doesn't exist
```

**Why:** First access to a key returns nil. Calling `.add()` on nil throws "doesNotUnderstand".

### ✅ Array Initialization
```supercollider
// GOOD: Initialize before loop
allMethods = Array.new;
dict.keysValuesDo { |key, val|
    allMethods = allMethods.add(val);
};

// BAD: Use uninitialized
dict.keysValuesDo { |key, val|
    allMethods = allMethods.add(val);  // ❌ allMethods is nil on first iteration
};
```

**Evidence:** Fixed crash in `LSPDatabase.sc:203` (2026-01-05).

### ✅ Function Returns in Dictionaries
```supercollider
// GOOD: Use if/else expressions (returns last value)
commands = (
    'supercollider.eval': { |params|
        if (params["source"].notNil) {
            var result = params["source"].interpret;
            ("result": result.asString)
        } {
            ("error": "No source provided")
        }
    }
);

// BAD: Use ^ (non-local return)
commands = (
    'supercollider.eval': { |params|
        if (params["source"].isNil) {
            ^("error": "No source")  // ❌ Returns provider object, not error dict
        };
        ^("result": params["source"].interpret.asString)
    }
);
```

**Evidence:** Caused "returning *itself*" warnings in ExecuteCommandProvider (2026-01-04).

**Why:** `^` (caret) is a non-local return - it returns from the enclosing method, not the function.
When the function is stored in a dictionary and called via `valueArray`, the `^` bypasses the return
value capture and returns the provider object itself instead of the intended result.

**Fix:** Use if/else expressions instead - they return the value of the last expression in each branch.

### ✅ Class Variable Initialization
```supercollider
// GOOD: initClass with accessors
TextDocumentProvider {
    classvar <pendingOpens, <pendingChanges, <initialized;

    *initClass {
        pendingOpens = Array.new;
        pendingChanges = Array.new;
        initialized = false;
    }
}

// BAD: Initialize in instance method
TextDocumentProvider {
    classvar pendingOpens;  // ❌ No accessor, no initClass

    init {
        pendingOpens = Array.new;  // ❌ May run after other code tries to access
    }
}
```

**Evidence:** Fixed DNUs in TextDocumentProvider (2026-01-06).

**Why:** Classvars are shared across all instances. They should be initialized once when the class
loads, not in instance methods. Use `*initClass` which runs automatically at class compile time.

### ✅ Race Condition Handling
```supercollider
// GOOD: Handle out-of-order messages
didChange { |uri, version, changes|
    var doc = LSPDocument.findByQUuid(uri);

    if (doc.isOpen.not) {
        Log.warning("Document % received change before open, forcing open", uri);
        doc.isOpen_(true);
    };

    changes.do(doc.applyChange(version, _));
}

// BAD: Assume documents always opened first
didChange { |uri, version, changes|
    var doc = LSPDocument.findByQUuid(uri);
    changes.do(doc.applyChange(version, _));  // ❌ Throws error if not open
}
```

**Evidence:** Fixed race condition in TextDocumentProvider (2026-01-05).

**Why:** LSP messages can arrive out of order (especially during startup). didChange might arrive
before didOpen is processed. Handle gracefully rather than throwing errors.

## Zed Extension Patterns

### ✅ Minimal Language Config
```toml
# GOOD: Only documented fields
name = "SuperCollider"
grammar = "supercollider"
path_suffixes = ["sc", "scd"]
line_comments = ["// "]
tab_size = 4
hard_tabs = false

# BAD: Undocumented fields
name = "SuperCollider"
grammar = "supercollider"
path_suffixes = ["sc", "scd"]
line_comments = ["// "]
tab_size = 4
hard_tabs = false
opt_into_language_servers = ["supercollider"]  # ❌ Breaks navigation
scope_opt_in_language_servers = ["supercollider"]  # ❌ Breaks navigation
```

**Evidence:** These exact fields prevented Zed from sending definition requests (2026-01-05).
Verified by `grep -i "definition" /tmp/sc_launcher_stdin.log` showing 0 requests with fields,
multiple requests after removal.

**Why:** These fields work for built-in Zed languages (TSX, Rust) but break extension-provided
languages. Zed's extension loading path differs from built-in language loading.

**Reference:** Compare with working extensions ([Erlang](https://github.com/zed-extensions/erlang),
[Elixir](https://github.com/zed-extensions/elixir)) - none use these fields.

### ✅ Dev Launcher Detection
```rust
// GOOD: Check for Cargo.toml (reliable file check)
fn dev_launcher_candidate(worktree: &zed::Worktree) -> Option<String> {
    if worktree.read_text_file("Cargo.toml").is_ok() {
        let root = worktree.root_path();
        Some(format!("{}/server/launcher/target/release/sc_launcher", root))
    } else {
        None
    }
}

// BAD: Check directory existence
if worktree.read_text_file("server/launcher").is_ok() { ... }  // ❌ read_text_file is for files
```

**Why:** `read_text_file()` is for files, not directories. Use a known file (like Cargo.toml) to
detect the development environment.

### ✅ Settings Merging
```rust
// GOOD: Deep merge user settings with defaults
fn merge_settings(base: &mut Value, overrides: &Value) {
    match (base, overrides) {
        (Value::Object(base_map), Value::Object(override_map)) => {
            for (key, value) in override_map {
                match base_map.get_mut(key) {
                    Some(base_value) => merge_settings(base_value, value),
                    None => { base_map.insert(key.clone(), value.clone()); }
                }
            }
        }
        (base_slot, override_value) => {
            *base_slot = override_value.clone();
        }
    }
}

// BAD: Shallow merge (loses nested settings)
let config = user_settings.or(default_settings);  // ❌ Can't merge nested objects
```

**Why:** User settings should override specific keys while preserving other defaults. Shallow merge
replaces entire objects, losing unspecified defaults.

## Launcher Patterns

### ✅ UDP Message Chunking
```rust
// GOOD: Chunk large messages
const MAX_UDP_SIZE: usize = 8192;

fn send_message(socket: &UdpSocket, message: &str) -> io::Result<()> {
    for chunk in message.as_bytes().chunks(MAX_UDP_SIZE) {
        socket.send(chunk)?;
    }
    Ok(())
}

// BAD: Send large messages whole
socket.send(message.as_bytes())?;  // ❌ Fails with "Message too long" for >8KB
```

**Why:** UDP has packet size limits. Messages larger than ~8KB fail with "Message too long" error.

**Evidence:** Fixed in launcher to handle large LSP responses (2025-10-16).

### ✅ Message Buffering
```rust
// GOOD: Buffer until sclang ready
if !lsp_ready {
    pending_messages.push(msg);
    return Ok(());
}
// Now safe to send
socket.send(&msg)?;

// BAD: Send immediately
socket.send(&msg)?;  // ❌ sclang not ready, message lost
```

**Why:** sclang takes 2-3 seconds to start and load LanguageServer.quark. Messages sent before
"***LSP READY***" marker are lost.

**Evidence:** Implemented in launcher initialization sequence (2025-10-16).

### ✅ Graceful Shutdown
```rust
// GOOD: Handle stdin close as shutdown signal
match stdin.read_line(&mut line) {
    Ok(0) => {
        eprintln!("stdin closed, shutting down");
        break;
    }
    // ...
}

// On exit, kill child process
if let Some(mut child) = sclang_process {
    let _ = child.kill();
}

// BAD: Ignore stdin close, leave orphans
loop {
    stdin.read_line(&mut line).ok();
    // ❌ Never breaks, sclang orphaned when Zed exits
}
```

**Why:** When Zed closes or restarts the language server, it closes stdin. This is the signal to
shut down gracefully and kill child processes.

## Tree-sitter Patterns

### ✅ Runnables Queries
```scheme
; GOOD: Specific captures with tags
((code_block) @code @run
  (#set! tag sc-eval))

((function_block) @code @run
  (#set! tag sc-eval))

; BAD: No tag (won't match tasks)
((code_block) @code @run)  ; ❌ No way to match this in tasks.json
```

**Why:** Tags connect tree-sitter captures to Zed tasks. `#set! tag sc-eval` allows task JSON to
specify `"tags": ["sc-eval"]` for matching.

### ✅ Highlight Queries
```scheme
; GOOD: Precise captures
(class_definition name: (identifier) @type)
(method_definition name: (identifier) @function.method)
(symbol) @string.special.symbol

; BAD: Too broad
(identifier) @variable  ; ❌ Highlights everything as variable
```

**Why:** Overly broad queries create incorrect highlighting. Be specific about context.

## Testing Patterns

### ✅ Verification After Changes
```bash
# GOOD: Check specific evidence of feature working
grep -i "definition" /tmp/sc_launcher_stdin.log  # For navigation
grep -i "error\|exception\|dnu" /tmp/sclang_post.log  # For Quark changes
curl -X POST -d "1 + 1" http://127.0.0.1:57130/eval  # For HTTP changes

# BAD: Assume it works
# Make change, don't verify  # ❌ May be broken
```

**Why:** Silent failures are common. Always verify with concrete evidence.

### ✅ Clean Test Environment
```bash
# GOOD: Kill and restart between major tests
pkill -9 sc_launcher; pkill -9 sclang
sleep 1
rm /tmp/sc_launcher_stdin.log /tmp/sclang_post.log
# Reopen .scd file in Zed

# BAD: Test without restart
# Change code → test immediately  # ❌ Old process still running, false results
```

**Why:** Processes keep running after code changes. Old binaries or state can hide bugs or create
false failures.

## File Organization

### ✅ Launcher Code Sections

Keep these sections distinct in `server/launcher/src/main.rs`:
- Args/config parsing (top)
- LSP bridge (stdio ↔ UDP)
- Message buffering
- HTTP server
- Process management
- Main function (bottom)

**Don't mix:** HTTP handling with LSP handling, or process management with message translation.

### ✅ Quark Provider Pattern

Each provider is a separate class:
- `TextDocumentProvider` - Document lifecycle
- `GotoDefinitionProvider` - Specific LSP capability
- `CompletionProvider` - Specific LSP capability
- etc.

All registered in `LSP.sc:initialize()`

**Don't:** Put multiple capabilities in one provider, or mix concerns.

## Error Handling

### ✅ Diagnostic Error Messages
```supercollider
// GOOD: Include context
Error("Document % received change before open, forcing open".format(uri)).warn;

// BAD: Vague errors
Error("Document not open").throw;  // ❌ What document? What operation?
```

**Why:** Debugging requires context. Which document? What was being attempted?

### ✅ Log Levels
```supercollider
// GOOD: Appropriate levels
Log.info("Handling: %", method);  // Normal operation
Log.warning("Queuing % until ready", method);  // Unusual but handled
Log.error("Failed to parse: %", params);  // Problem needing attention

// BAD: Everything at one level
Log.info("ERROR: Failed to parse");  // ❌ Wrong level, harder to filter
```

**Why:** Log levels enable filtering. Use them correctly for effective debugging.

## Quark Development Workflow

### ✅ Edit → Copy → Kill → Test
```bash
# 1. Edit in vendored location
vim server/quark/LanguageServer.quark/Providers/ExecuteCommandProvider.sc

# 2. Copy to system location
cp server/quark/LanguageServer.quark/Providers/ExecuteCommandProvider.sc \
   ~/Library/Application\ Support/SuperCollider/downloaded-quarks/LanguageServer/Providers/

# 3. Kill sclang to force reload
pkill -9 sclang

# 4. Test (reopen .scd file in Zed)
grep -i "error" /tmp/sclang_post.log

# BAD: Edit system location directly
vim ~/Library/Application\ Support/SuperCollider/.../File.sc  # ❌ Changes not in git
```

**Why:** Vendored quark is in git. System location is where sclang loads from. Must sync both.
