# Codebase Improvements

Comprehensive list of potential improvements to the zed-supercollider project, organized by component and priority.

---

## 🔴 High Priority

### Configuration & Stability

#### 1. Remove Custom Notification Spam
**File:** `server/quark/LanguageServer.quark/` (wherever `supercollider/serverStatus` is sent)

**Issue:** Zed logs fill with "unhandled notification supercollider/serverStatus"

**Solutions:**
- Option A: Remove the custom notification entirely if not used
- Option B: Only send when Zed explicitly subscribes
- Option C: Switch to standard LSP progress notifications (`window/workDoneProgress`)

**Impact:** Cleaner logs, better Zed integration

---

#### 2. Graceful Shutdown Handling
**File:** `server/launcher/src/main.rs`

**Issue:** Logs show "timeout waiting for language server supercollider to shutdown"

**Improvements:**
```rust
// Add proper shutdown sequence:
// 1. Stop accepting new requests
// 2. Finish pending requests (with timeout)
// 3. Close UDP socket
// 4. Send exit to sclang
// 5. Wait for sclang process with timeout
// 6. Force kill if necessary
```

**Current Code Location:** Shutdown handling around UDP and sclang process management

**Impact:** No zombie processes, clean restarts

---

#### 3. Better Error Messages
**Multiple Files:** Extension, launcher, quark

**Current State:** Generic errors like "launcher not found"

**Improvements:**
```rust
// Extension (src/lib.rs)
if cmd_path.is_none() {
    return Err(formatdoc! {"
        SuperCollider LSP launcher not found.

        Troubleshooting:
        1. Install sc_launcher: cargo install sc_launcher
        2. Or set path in settings:
           \"lsp\": {{
             \"supercollider\": {{
               \"binary\": {{
                 \"path\": \"/path/to/sc_launcher\"
               }}
             }}
           }}
        3. Or open extension source directory (auto-detects dev mode)

        For help: https://github.com/zed-supercollider/issues
    "}.into());
}
```

**Impact:** Users can self-diagnose issues

---

### Language Server Protocol Compliance

#### 4. Proper Response Format for Definition Requests
**File:** `server/quark/LanguageServer.quark/Providers/GotoDefinitionProvider.sc`

**Current:** Returns `Location[]`

**LSP Spec Allows:**
- `Location` - single location
- `Location[]` - multiple locations
- `LocationLink[]` - with context (preferred)

**Improvement:**
```supercollider
// Use LocationLink for better UX
// Shows origin and target ranges
LocationLink {
    originSelectionRange: <range of symbol clicked>,
    targetUri: <file path>,
    targetRange: <full definition range>,
    targetSelectionRange: <name range in definition>
}
```

**Impact:** Better preview in Zed, shows context

**Reference:** https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#locationLink

---

#### 5. Handle Empty/Null Responses Correctly
**File:** `server/quark/LanguageServer.quark/Providers/*.sc`

**Issue:** Some providers might return `nil` instead of proper empty responses

**Fix:**
```supercollider
// Ensure we return proper JSON-RPC response
// Success with no results: { "id": X, "result": null }
// Success with results: { "id": X, "result": [...] }
// Error: { "id": X, "error": { "code": -32601, "message": "..." } }
```

**Impact:** LSP protocol compliance

---

## 🟡 Medium Priority

### Performance Optimizations

#### 6. LSPDatabase Indexing Performance
**File:** `server/quark/LanguageServer.quark/LSPDatabase.sc`

**Current:** Rebuilds entire index on changes

**Improvements:**
```supercollider
// Add incremental indexing
LSPDatabase {
    classvar indexVersion;
    classvar lastIndexTime;

    *needsReindex {
        // Check if class library changed since last index
        ^(Date.getDate.rawSeconds - lastIndexTime) > 60
    }

    *incrementalUpdate { |changedFiles|
        // Only re-index changed files
        // Keep rest of index intact
    }
}
```

**Benchmark Current Performance:**
```supercollider
var start = Main.elapsedTime;
LSPDatabase.buildIndex;
var elapsed = Main.elapsedTime - start;
"Full index build: %ms".format(elapsed * 1000).postln;
```

**Impact:** Faster updates, lower CPU usage

---

#### 7. Cache Frequently Requested Definitions
**File:** `server/quark/LanguageServer.quark/LSPDatabase.sc`

**Add LRU Cache:**
```supercollider
LSPDatabase {
    classvar definitionCache; // Dictionary: symbol -> Location[]
    classvar cacheMaxSize = 1000;
    classvar cacheHits = 0;
    classvar cacheMisses = 0;

    *findDefinitions { |symbol|
        var cached = definitionCache[symbol];
        if (cached.notNil) {
            cacheHits = cacheHits + 1;
            ^cached;
        };
        cacheMisses = cacheMisses + 1;
        // ... do expensive lookup ...
        definitionCache[symbol] = result;
        ^result;
    }
}
```

**Impact:** Sub-millisecond responses for common classes/methods

---

#### 8. Launcher Startup Optimization
**File:** `server/launcher/src/main.rs`

**Current:** ~2-3 seconds to start sclang + compile class library

**Investigate:**
- Can we pre-warm sclang in background?
- Can we use sclang's daemon mode?
- Can we lazy-load class library?

**Measurement:**
```rust
use std::time::Instant;

let start = Instant::now();
// Start sclang...
let sclang_start = start.elapsed();
eprintln!("sclang process started in {:?}", sclang_start);

// Wait for LSP READY...
let ready_time = start.elapsed();
eprintln!("LSP ready in {:?}", ready_time);
```

**Impact:** Faster editor startup

---

#### 9. Message Batching for UDP
**File:** `server/launcher/src/main.rs`

**Current:** Each LSP message sent individually over UDP

**Improvement:**
```rust
// Batch small messages together
// Reduces UDP overhead for rapid-fire requests
struct MessageBatcher {
    pending: Vec<Message>,
    last_flush: Instant,
    max_batch_size: usize,
    max_batch_delay: Duration,
}

impl MessageBatcher {
    fn add(&mut self, msg: Message) {
        self.pending.push(msg);
        if self.should_flush() {
            self.flush();
        }
    }

    fn should_flush(&self) -> bool {
        self.pending.len() >= self.max_batch_size ||
        self.last_flush.elapsed() >= self.max_batch_delay
    }
}
```

**Impact:** Lower latency for rapid operations (typing, navigation)

---

### Code Quality & Maintainability

#### 10. Add Comprehensive Logging
**All Components**

**Extension (src/lib.rs):**
```rust
// Replace eprintln! with proper log levels
use log::{debug, info, warn, error};

info!("SuperCollider extension initialized");
debug!("Language server command: {} {:?}", cmd.command, cmd.args);
warn!("Launcher not found in PATH, trying dev mode");
error!("Failed to start language server: {}", err);
```

**Launcher:**
```rust
// Add structured logging with context
use tracing::{info, debug, warn, error, instrument};

#[instrument(level = "debug")]
fn handle_lsp_message(msg: &LspMessage) {
    debug!(method = %msg.method, "Handling LSP message");
    // ...
}
```

**Quark:**
```supercollider
// Use Log() consistently
Log('LSPDatabase').info("Building index for % classes", allClasses.size);
Log('GotoDefinition').debug("Searching for definition of %", symbol);
```

**Impact:** Easier debugging, better production monitoring

---

#### 11. Add Configuration Validation
**File:** `languages/SuperCollider/config.toml`

**Create Validation Tool:**
```bash
#!/bin/bash
# scripts/validate-config.sh

echo "Validating extension configuration..."

# Check for invalid fields
if grep -q "opt_into_language_servers" languages/SuperCollider/config.toml; then
    echo "❌ ERROR: Invalid field 'opt_into_language_servers' found"
    echo "   This field breaks Zed extensions. Remove it."
    exit 1
fi

# Check required fields
if ! grep -q 'name = "SuperCollider"' languages/SuperCollider/config.toml; then
    echo "❌ ERROR: Missing required field 'name'"
    exit 1
fi

echo "✅ Configuration valid"
```

**CI Integration:** Run in GitHub Actions

**Impact:** Prevent config regressions

---

#### 12. Type Safety Improvements
**File:** `server/launcher/src/main.rs`

**Current:** Lots of string parsing and JSON manipulation

**Improvement:**
```rust
// Use strongly-typed LSP structures
use lsp_types::{
    GotoDefinitionParams, GotoDefinitionResponse,
    Location, LocationLink, Position, Range,
};

fn handle_definition_request(
    params: GotoDefinitionParams
) -> Result<GotoDefinitionResponse, Error> {
    // Type-safe request handling
    let position = params.text_document_position_params.position;
    let uri = params.text_document_position_params.text_document.uri;

    // ... forward to sclang ...

    // Type-safe response construction
    Ok(GotoDefinitionResponse::Link(locations))
}
```

**Dependencies:**
```toml
[dependencies]
lsp-types = "0.95"
serde_json = "1.0"
```

**Impact:** Catch errors at compile time, better IDE support

---

## 🟢 Nice to Have

### Developer Experience

#### 13. Hot Reload for Quark Development
**File:** `server/quark/LanguageServer.quark/`

**Problem:** Need to restart sclang to test quark changes

**Solution:**
```supercollider
// Add reload capability
LSPConnection {
    *reload {
        "Reloading LanguageServer.quark...".postln;

        // Clear provider cache
        this.providers = nil;

        // Recompile quark files
        thisProcess.interpreter.executeFile(
            "~/path/to/LanguageServer.quark/LSP.sc"
        );

        // Re-initialize
        this.init;

        "Reload complete".postln;
    }
}
```

**Usage:** `/reload` command during development

**Impact:** Faster iteration

---

#### 14. Development Mode Indicators
**File:** `src/lib.rs`

**Add Visual Feedback:**
```rust
fn language_server_command(...) -> Result<Command> {
    let is_dev_mode = dev_launcher_candidate(worktree).is_some();

    if is_dev_mode {
        // Could send notification to Zed
        eprintln!("🔧 [DEV MODE] Using local launcher");
        eprintln!("   Launcher: {}", cmd.command);
        eprintln!("   Quark: {}/server/quark/LanguageServer.quark", worktree.root_path());
    }

    Ok(cmd)
}
```

**Impact:** Clear when testing local changes

---

#### 15. Interactive Debugging Console
**New File:** `server/launcher/src/debug.rs`

**Add Debug Socket:**
```rust
// Listen on localhost:9999 for debug commands
// Telnet in: `telnet localhost 9999`

Commands:
- stats: Show message counts, timing stats
- dump: Dump current state
- trace on/off: Toggle detailed tracing
- cache: Show definition cache stats
```

**Impact:** Debug production issues without restarting

---

### Testing Infrastructure

#### 16. LSP Protocol Compliance Tests
**New File:** `server/launcher/tests/lsp_compliance_tests.rs`

```rust
#[cfg(test)]
mod tests {
    use lsp_types::*;

    #[test]
    fn test_initialize_request() {
        let launcher = start_test_launcher();

        let params = InitializeParams {
            // ...
        };

        let response = launcher.send_request("initialize", params);
        assert!(response.capabilities.definition_provider.is_some());
    }

    #[test]
    fn test_definition_request_format() {
        // Verify response matches LSP spec
    }
}
```

**Impact:** Catch protocol violations early

---

#### 17. Integration Tests with Real sclang
**New File:** `tests/integration_test.rs`

```rust
#[test]
#[ignore] // Requires sclang installed
fn test_full_definition_lookup() {
    let mut launcher = spawn_launcher();

    // 1. Initialize
    launcher.initialize();

    // 2. Open document
    launcher.did_open("test.sc", "SinOsc.ar");

    // 3. Request definition
    let locations = launcher.goto_definition(Position {
        line: 0,
        character: 0,
    });

    // 4. Verify we got SinOsc class definition
    assert_eq!(locations.len(), 1);
    assert!(locations[0].uri.path().contains("SinOsc.sc"));
}
```

**Run with:** `cargo test -- --ignored`

**Impact:** Catch regressions in real scenarios

---

#### 18. Benchmark Suite
**New File:** `benches/performance.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_definition_lookup(c: &mut Criterion) {
    let launcher = setup_launcher();

    c.bench_function("definition_lookup_common_class", |b| {
        b.iter(|| {
            launcher.goto_definition(black_box("SinOsc"))
        })
    });

    c.bench_function("definition_lookup_rare_class", |b| {
        b.iter(|| {
            launcher.goto_definition(black_box("ObscureClass"))
        })
    });
}

criterion_group!(benches, bench_definition_lookup);
criterion_main!(benches);
```

**Run with:** `cargo bench`

**Impact:** Track performance regressions

---

### Documentation

#### 19. Architecture Documentation
**New File:** `docs/ARCHITECTURE.md`

```markdown
# Architecture

## Overview
3-layer bridge: Zed Extension (WASM) ↔ Launcher (Rust) ↔ sclang (UDP)

## Message Flow

1. User triggers go-to-definition in Zed
2. Zed sends LSP request to extension WASM
3. Extension's language_server_command() provides launcher path
4. Zed starts launcher process (stdio)
5. Launcher receives LSP JSON via stdin
6. Launcher forwards to sclang via UDP
7. sclang processes in LanguageServer.quark
8. Response flows back: UDP → Launcher → stdout → Zed

## Component Details
[Detailed explanation of each layer]
```

**Impact:** New contributors can understand system quickly

---

#### 20. API Documentation
**Files:** All Rust and SuperCollider code

**Rust:**
```rust
/// Handles the initialization of the language server.
///
/// This method is called once when Zed first connects to the language server.
/// It performs the following steps:
/// 1. Starts the sclang process
/// 2. Establishes UDP communication
/// 3. Waits for "***LSP READY***" message
/// 4. Forwards buffered messages
///
/// # Arguments
/// * `params` - Standard LSP InitializeParams
///
/// # Returns
/// * `Ok(InitializeResult)` - Server capabilities
/// * `Err(Error)` - If sclang fails to start
fn initialize(&mut self, params: InitializeParams) -> Result<InitializeResult>
```

**SuperCollider:**
```supercollider
// Finds the definition(s) of a symbol.
//
// This method searches through the class library to find where a symbol
// (class, method, variable) is defined. It supports:
// - Class definitions
// - Instance method definitions
// - Class method definitions
//
// Arguments:
//   symbol - String, the symbol to search for
//   context - Optional, the file context for scoped search
//
// Returns:
//   Array of Location objects, empty if not found
```

**Generate with:** `cargo doc --open`

**Impact:** Better maintainability

---

### User Experience

#### 21. Progress Notifications
**File:** `server/quark/LanguageServer.quark/LSP.sc`

**Use Standard LSP Progress:**
```supercollider
LSPConnection {
    *reportProgress { |title, message, percentage|
        this.sendNotification(
            method: '$/progress',
            params: (
                token: 'sc-index',
                value: (
                    kind: 'report',
                    title: title,
                    message: message,
                    percentage: percentage
                )
            )
        );
    }
}

// Usage during indexing:
LSPDatabase {
    *buildIndex {
        LSPConnection.reportProgress(
            "Indexing SuperCollider",
            "Building class library index...",
            0
        );

        // ... build index ...

        LSPConnection.reportProgress(
            "Indexing SuperCollider",
            "Index complete",
            100
        );
    }
}
```

**Impact:** User sees "Indexing SuperCollider..." in Zed status bar

---

#### 22. Configuration Schema
**New File:** `schema/supercollider-config.json`

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "SuperCollider LSP Configuration",
  "type": "object",
  "properties": {
    "sclangPath": {
      "type": "string",
      "description": "Path to sclang executable",
      "default": "/Applications/SuperCollider.app/Contents/MacOS/sclang"
    },
    "enableHover": {
      "type": "boolean",
      "description": "Enable hover documentation",
      "default": true
    },
    "maxIndexingTime": {
      "type": "number",
      "description": "Maximum time for indexing in milliseconds",
      "default": 5000
    }
  }
}
```

**Register in extension.toml:**
```toml
[language_servers.supercollider]
language = "SuperCollider"
config_schema = "schema/supercollider-config.json"
```

**Impact:** Auto-complete for settings in Zed

---

#### 23. Inline Diagnostics from sclang
**File:** `server/quark/LanguageServer.quark/`

**Add DiagnosticsProvider:**
```supercollider
DiagnosticsProvider : LSPProvider {
    *methodNames { ^["textDocument/didChange"] }

    onReceived { |method, params|
        var uri = params["textDocument"]["uri"];
        var text = params["contentChanges"][0]["text"];

        // Try to parse/compile
        var errors = this.checkSyntax(text);

        // Send diagnostics to client
        LSPConnection.sendNotification(
            method: 'textDocument/publishDiagnostics',
            params: (
                uri: uri,
                diagnostics: errors.collect { |err|
                    (
                        range: err.range,
                        severity: 1, // Error
                        message: err.message
                    )
                }
            )
        );
    }
}
```

**Impact:** Red squiggles for syntax errors before evaluation

---

### Feature Enhancements

#### 24. Workspace Symbol Search
**File:** `server/quark/LanguageServer.quark/Providers/WorkspaceSymbolProvider.sc`

**Improve Current Implementation:**
```supercollider
WorkspaceSymbolProvider {
    onReceived { |method, params|
        var query = params["query"];
        var symbols = [];

        // Fuzzy match like Cmd+T in editors
        symbols = LSPDatabase.allClasses.select { |cls|
            this.fuzzyMatch(cls.name.asString, query)
        }.collect { |cls|
            (
                name: cls.name,
                kind: 5, // Class
                location: (
                    uri: "file://" ++ cls.filenameSymbol,
                    range: cls.sourceRange
                ),
                // Show inheritance in detail
                detail: cls.superclass !? { "extends " ++ cls.superclass.name }
            )
        };

        ^symbols;
    }

    fuzzyMatch { |text, pattern|
        // Implement fuzzy matching algorithm
        // "SinOsc" matches "so", "sinoc", "sinosc"
    }
}
```

**Impact:** Cmd+T to jump to any class quickly

---

#### 25. Signature Help (Parameter Hints)
**File:** `server/quark/LanguageServer.quark/Providers/SignatureHelpProvider.sc`

**Show Method Parameters:**
```supercollider
SignatureHelpProvider {
    onReceived { |method, params|
        var position = params["position"];
        var uri = params["textDocument"]["uri"];

        // Parse context to find method call
        var methodName = this.getMethodAtPosition(uri, position);
        var signatures = this.getMethodSignatures(methodName);

        ^(
            signatures: signatures.collect { |sig|
                (
                    label: sig.label,
                    documentation: sig.doc,
                    parameters: sig.params.collect { |param|
                        (
                            label: param.name,
                            documentation: param.doc
                        )
                    }
                )
            },
            activeSignature: 0,
            activeParameter: this.getCurrentParameter(position)
        );
    }
}
```

**Impact:** See parameter names and docs while typing

---

## 🔵 Infrastructure

#### 26. Continuous Integration
**New File:** `.github/workflows/ci.yml`

```yaml
name: CI

on: [push, pull_request]

jobs:
  test-launcher:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run tests
        run: |
          cd server/launcher
          cargo test --verbose
      - name: Run clippy
        run: cargo clippy -- -D warnings
      - name: Check formatting
        run: cargo fmt -- --check

  test-extension:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install wasm32-wasi target
        run: rustup target add wasm32-wasi
      - name: Build extension
        run: cargo build --target wasm32-wasi
      - name: Run config validation
        run: ./scripts/validate-config.sh

  test-quark:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install SuperCollider
        run: brew install supercollider
      - name: Run quark tests
        run: sclang tests/run_tests.scd
```

**Impact:** Catch issues before merge

---

#### 27. Release Automation
**New File:** `.github/workflows/release.yml`

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build-and-release:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: windows-latest
            target: x86_64-pc-windows-msvc
    steps:
      - uses: actions/checkout@v3
      - name: Build launcher
        run: |
          cd server/launcher
          cargo build --release --target ${{ matrix.target }}
      - name: Create release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            server/launcher/target/${{ matrix.target }}/release/sc_launcher*
```

**Impact:** Automated binary releases

---

#### 28. Extension Marketplace Submission
**File:** `README.md` (polish for submission)

**Checklist:**
- [ ] Clear description and features list
- [ ] Installation instructions
- [ ] Screenshots/GIFs of features
- [ ] License file (MIT/Apache-2.0)
- [ ] Contributing guidelines
- [ ] Issue templates

**Submit to:** https://github.com/zed-industries/extensions

**Impact:** Wider user adoption

---

## Priority Matrix

### Must Have (Blockers for Production)
1. Remove custom notification spam
2. Better error messages
3. Graceful shutdown handling
4. Configuration validation

### Should Have (Quality of Life)
5. Proper LSP response formats
6. Comprehensive logging
7. LSPDatabase performance
8. Definition caching

### Nice to Have (Polish)
9. Progress notifications
10. Signature help
11. Workspace symbol fuzzy search
12. Hot reload for development

### Future Exploration
13. Inline diagnostics
14. Debug console
15. Semantic highlighting
16. Integration with scsynth

---

## Implementation Order Suggestion

### Phase 1: Stability (Week 1-2)
- Remove custom notification spam
- Add graceful shutdown
- Improve error messages
- Add config validation
- Set up CI

### Phase 2: Performance (Week 3-4)
- LSPDatabase caching
- Message batching
- Startup optimization
- Add benchmarks

### Phase 3: Developer Experience (Week 5-6)
- Comprehensive logging
- Architecture docs
- Integration tests
- Hot reload support

### Phase 4: User Features (Week 7-8)
- Progress notifications
- Signature help
- Workspace symbol improvements
- Configuration schema

### Phase 5: Polish & Release (Week 9-10)
- Fix all remaining issues
- Documentation polish
- Extension marketplace submission
- Community announcement

---

## Metrics to Track

### Code Quality
- Test coverage: Target 80%+
- Clippy warnings: 0
- Documentation coverage: 90%+

### Performance
- Startup time: <2s (current: 2-3s)
- Definition lookup: <100ms p95
- Memory usage: <50MB steady state

### Reliability
- Crash rate: 0 per 1000 operations
- LSP protocol compliance: 100%
- Successful shutdown rate: 100%

### User Satisfaction
- Extension rating: 4.5+/5
- Issue close rate: >90%
- Feature request implementation: 50%+
