---
title: "Commands Reference"
created: 2026-01-05
updated: 2026-01-05
purpose: "Complete reference of build, test, debug, and deployment commands for the extension"
---

# Commands Reference

All commands for building, testing, and debugging the extension.

## Build

### Extension (WASM)
The extension builds automatically when you reload in Zed. No cargo command needed.

```bash
# In Zed: Extensions panel → Rebuild
# Or: Cmd+Shift+P → "zed: reload extensions"
```

**When to rebuild:**
- After editing `src/lib.rs`
- After editing tree-sitter queries in `languages/SuperCollider/`
- After editing `extension.toml`

**Output:** Extension is built to WASM and loaded by Zed

### Launcher (Native Binary)
The launcher must be built separately with cargo.

```bash
cd server/launcher
cargo build --release

# Output: server/launcher/target/release/sc_launcher
```

**When to rebuild:**
- After editing `server/launcher/src/main.rs`
- After editing `server/launcher/Cargo.toml`

## Test

### Smoke Test (Manual)
```bash
# 1. Start launcher manually
./server/launcher/target/release/sc_launcher --mode lsp

# 2. In another terminal - test HTTP
curl -X POST -d "1 + 1" http://127.0.0.1:57130/eval

# Expected output: {"result":"2"}
```

### In-Editor Test
```bash
# 1. Restart Zed (or reload extensions)
# 2. Open test_eval.scd
# 3. Click play button next to (1 + 1)
# 4. Check /tmp/sclang_post.log for "> 2"
tail -f /tmp/sclang_post.log
```

### Verify Navigation
```bash
# After config changes, verify definition requests are sent
grep -i "definition" /tmp/sc_launcher_stdin.log

# Should show multiple requests:
# {"jsonrpc":"2.0","id":4,"method":"textDocument/definition",...}
# {"jsonrpc":"2.0","id":6,"method":"textDocument/definition",...}

# If empty, navigation is broken (check config.toml)
```

### Verify Evaluation
```bash
# Test HTTP endpoint directly
curl -X POST -d "1 + 1" http://127.0.0.1:57130/eval

# Expected: {"result":"2"}

# If connection refused: launcher not running or wrong port
```

### Verify Server Health
```bash
# Check for errors in sclang
grep -i "error\|exception\|dnu" /tmp/sclang_post.log

# Should be empty or only old errors (before your changes)

# Check for LSP activity
grep -c "textDocument/" /tmp/sc_launcher_stdin.log

# Should be > 0 if LSP is working
```

## Debug

### Watch LSP Traffic
```bash
# Watch all messages from Zed to Launcher
tail -f /tmp/sc_launcher_stdin.log

# Watch sclang output (eval results, errors)
tail -f /tmp/sclang_post.log

# Watch Zed internal logs
tail -f ~/Library/Logs/Zed/Zed.log | grep -i supercollider
```

### Search Logs
```bash
# Check what LSP requests are being sent
grep -i "textDocument/" /tmp/sc_launcher_stdin.log | cut -d':' -f2 | sort | uniq

# Check for specific feature
grep -i "definition" /tmp/sc_launcher_stdin.log
grep -i "completion" /tmp/sc_launcher_stdin.log
grep -i "hover" /tmp/sc_launcher_stdin.log
grep -i "references" /tmp/sc_launcher_stdin.log

# Check for errors in sclang
grep -i "error" /tmp/sclang_post.log | tail -10
grep -i "exception" /tmp/sclang_post.log | tail -10
grep -i "doesnotunderstand\|dnu" /tmp/sclang_post.log | tail -10

# Check for warnings
grep -i "warning" /tmp/sclang_post.log | tail -10
```

### Force Restart Everything
```bash
# Kill all SuperCollider processes
pkill -9 sc_launcher
pkill -9 sclang
pkill -9 scsynth

# Wait a moment
sleep 1

# Clear logs
rm -f /tmp/sc_launcher_stdin.log
rm -f /tmp/sclang_post.log

# Restart Zed or reload extensions
# Then reopen .scd file
```

### Test HTTP Endpoints
```bash
# Health check
curl http://127.0.0.1:57130/health

# Evaluate code
curl -X POST -d "1 + 1" http://127.0.0.1:57130/eval

# Stop sounds (CmdPeriod)
curl -X POST http://127.0.0.1:57130/stop

# Boot server
curl -X POST http://127.0.0.1:57130/boot

# Recompile class library
curl -X POST http://127.0.0.1:57130/recompile

# Quit server
curl -X POST http://127.0.0.1:57130/quit
```

### Debug LSP Issue
See `.ai/prompts/debug-lsp-issue.md` for detailed diagnostic steps.

Quick checks:
```bash
# 1. Is LSP running?
tail ~/Library/Logs/Zed/Zed.log | grep -i "starting language server"

# 2. Are requests being sent?
tail /tmp/sc_launcher_stdin.log

# 3. Any sclang errors?
grep -i "error" /tmp/sclang_post.log | tail -5
```

## Deploy

### Copy Quark Changes to System
After editing files in `server/quark/LanguageServer.quark/`:

```bash
# Copy single file
cp server/quark/LanguageServer.quark/Providers/ExecuteCommandProvider.sc \
   ~/Library/Application\ Support/SuperCollider/downloaded-quarks/LanguageServer/Providers/

# Or copy entire directory
cp -r server/quark/LanguageServer.quark/* \
   ~/Library/Application\ Support/SuperCollider/downloaded-quarks/LanguageServer/

# Force sclang to reload
pkill -9 sclang

# Reopen .scd file in Zed
```

### Install Extension in Zed
```bash
# Development install (from source)
# In Zed: Extensions → Install Dev Extension → select this repo

# After code changes:
# Cmd+Shift+P → "zed: reload extensions"
```

## Common Operations

### After Quark Changes
```bash
# 1. Copy to system location
cp server/quark/LanguageServer.quark/File.sc \
   ~/Library/Application\ Support/SuperCollider/downloaded-quarks/LanguageServer/

# 2. Kill sclang
pkill -9 sclang

# 3. Check for errors
tail -f /tmp/sclang_post.log

# 4. Test in Zed (reopen .scd file)
```

### After Launcher Changes
```bash
# 1. Rebuild
cd server/launcher && cargo build --release

# 2. Kill old launcher
pkill -9 sc_launcher

# 3. Zed will restart it automatically when you reopen .scd file

# 4. Verify it's running
curl http://127.0.0.1:57130/health
```

### After Extension Changes
```bash
# 1. In Zed: Cmd+Shift+P → "zed: reload extensions"

# 2. Reopen .scd file

# 3. Check logs
tail ~/Library/Logs/Zed/Zed.log | grep -i supercollider
```

### After Config Changes
```bash
# 1. Commit the change to git (Zed uses committed version)
git add languages/SuperCollider/config.toml
git commit -m "Update config"

# 2. Reload extension
# Cmd+Shift+P → "zed: reload extensions"

# 3. Verify navigation works
grep -i "definition" /tmp/sc_launcher_stdin.log
```

## Verification Commands

### Full Health Check
```bash
# LSP requests being sent?
echo "LSP requests:"
grep -c "textDocument/" /tmp/sc_launcher_stdin.log

# Sclang errors?
echo "Sclang errors:"
grep -i "error\|exception\|dnu" /tmp/sclang_post.log | wc -l

# HTTP server running?
echo "HTTP server:"
curl -s http://127.0.0.1:57130/health && echo "OK" || echo "NOT RUNNING"

# Launcher running?
echo "Launcher process:"
ps aux | grep sc_launcher | grep -v grep
```

### Feature-Specific Tests
```bash
# Go-to-definition working?
grep -c "textDocument/definition" /tmp/sc_launcher_stdin.log
# Should be > 0

# Completions working?
grep -c "textDocument/completion" /tmp/sc_launcher_stdin.log
# Should be > 0

# Eval working?
curl -X POST -d "2 + 2" http://127.0.0.1:57130/eval
# Should return: {"result":"4"}
```

## Log File Locations

| Log | What It Contains |
|-----|------------------|
| `/tmp/sc_launcher_stdin.log` | LSP messages from Zed → Launcher |
| `/tmp/sclang_post.log` | sclang stdout/stderr (eval results, errors) |
| `~/Library/Logs/Zed/Zed.log` | Zed internal logs (search for "supercollider") |

## Troubleshooting Commands

### LSP Not Starting
```bash
# Check if launcher binary exists
ls -la server/launcher/target/release/sc_launcher

# Check Zed settings for launcher path
cat ~/.config/zed/settings.json | grep -A 5 supercollider

# Try manual start
./server/launcher/target/release/sc_launcher --mode lsp
# Should see: "HTTP eval server listening on..."
```

### Evaluation Not Working
```bash
# Is HTTP server listening?
lsof -i :57130

# If not, is launcher running?
ps aux | grep sc_launcher

# Test directly
curl -v -X POST -d "1+1" http://127.0.0.1:57130/eval
```

### Quark Issues
```bash
# Check if LanguageServer.quark installed
# In SuperCollider IDE:
Quarks.installed

# Should see "LanguageServer" in the list

# Check for sclang errors
tail -50 /tmp/sclang_post.log | grep -i "error\|exception"
```

### Clean Slate Reset
```bash
# Nuclear option: kill everything and clear all state

pkill -9 sc_launcher
pkill -9 sclang
pkill -9 scsynth

rm -f /tmp/sc_launcher_stdin.log
rm -f /tmp/sclang_post.log

# Restart Zed
# Reopen .scd file
```
