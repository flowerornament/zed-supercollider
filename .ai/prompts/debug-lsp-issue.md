# Prompt: Debug LSP Issue

Use this when LSP features aren't working (completions, navigation, hover, etc.)

## Quick Diagnostic

Run these commands first for immediate triage:

```bash
# 1. Is LSP running?
tail -20 ~/Library/Logs/Zed/Zed.log | grep -i "supercollider\|language server"

# 2. Are requests being sent?
tail -20 /tmp/sc_launcher_stdin.log

# 3. Any recent errors?
grep -i "error\|exception\|dnu" /tmp/sclang_post.log | tail -10
```

## Systematic Diagnosis

### Step 1: Verify LSP Process Running

```bash
# Check Zed logs for LSP startup
tail -50 ~/Library/Logs/Zed/Zed.log | grep -i "starting language server"

# Should see: "INFO [lsp] starting language server process. name: supercollider"
```

**If not found:**
- LSP didn't start
- Check Zed settings for launcher path
- Verify binary exists: `ls -la server/launcher/target/release/sc_launcher`

### Step 2: Verify Launcher Receiving Messages

```bash
# Check launcher stdin log
tail -50 /tmp/sc_launcher_stdin.log

# Should see:
# - initialize request
# - textDocument/didOpen
# - textDocument/didChange
# - Feature requests (definition, completion, etc.)
```

**If empty or very short:**
- Launcher started but not receiving messages
- Check Zed LSP logs for connection errors
- Try manual start: `./server/launcher/target/release/sc_launcher --mode lsp`

### Step 3: Verify sclang Started Clean

```bash
# Check for errors during startup
grep -i "error\|exception" /tmp/sclang_post.log | head -20

# Should be empty or only old errors (before current session)

# Check for DNUs (doesNotUnderstand)
grep -i "doesnotunderstand\|dnu" /tmp/sclang_post.log | tail -10

# Should be empty
```

**If errors found:**
- Quark code has bugs
- Check specific error message
- Look for class or method name in error
- May need to fix Quark code

### Step 4: Verify Capability Advertisement

```bash
# Check initialize response
grep -A 100 '"result".*"capabilities"' /tmp/sc_launcher_stdin.log | head -120

# Should see:
# "capabilities": {
#   "definitionProvider": true,
#   "completionProvider": {...},
#   "referencesProvider": true,
#   ...
# }
```

**If missing capabilities:**
- Quark didn't register provider
- Check LSP.sc initialization
- Check provider files exist in Quark

### Step 5: Check Specific Feature Requests

```bash
# For go-to-definition issues:
grep -i "textDocument/definition" /tmp/sc_launcher_stdin.log

# For completion issues:
grep -i "textDocument/completion" /tmp/sc_launcher_stdin.log

# For hover issues:
grep -i "textDocument/hover" /tmp/sc_launcher_stdin.log

# For references issues:
grep -i "textDocument/references" /tmp/sc_launcher_stdin.log
```

**Expected:** Multiple requests for working features

## Common Issues & Fixes

### Issue: No LSP Requests At All

**Symptoms:**
- `/tmp/sc_launcher_stdin.log` is empty or has only `initialize`
- No completions, no navigation, nothing works

**Likely Cause:** Launcher not starting or crashing immediately

**Fix:**
```bash
# 1. Verify binary exists
ls -la server/launcher/target/release/sc_launcher

# 2. Try manual start to see errors
./server/launcher/target/release/sc_launcher --mode lsp

# 3. Check Zed settings
cat ~/.config/zed/settings.json | grep -A 10 supercollider

# 4. Rebuild launcher
cd server/launcher && cargo build --release
```

### Issue: Requests Sent But No Responses

**Symptoms:**
- Requests appear in stdin log
- Features don't work in editor
- No error messages

**Likely Cause:** sclang crash or Quark error

**Fix:**
```bash
# 1. Check sclang log for errors
grep -i "error\|exception\|dnu" /tmp/sclang_post.log

# 2. Look for provider registration
grep -i "found providers" /tmp/sclang_post.log

# 3. Verify LanguageServer.quark installed
# In SuperCollider IDE: Quarks.installed

# 4. Check quark version
ls -la ~/Library/Application\ Support/SuperCollider/downloaded-quarks/LanguageServer/
```

### Issue: Navigation Works, Completions Don't (or vice versa)

**Symptoms:**
- Some features work
- Others don't

**Likely Causes:**
1. Feature not advertised in capabilities
2. Provider not implemented in Quark
3. Config issue (unlikely if some features work)

**Fix:**
```bash
# 1. Check if feature advertised
grep -A 100 "capabilities" /tmp/sc_launcher_stdin.log | grep -i "completion\|hover\|definition"

# 2. Check if requests being sent
grep -i "textDocument/completion" /tmp/sc_launcher_stdin.log

# 3. If not sent: Zed client issue or config issue
# If sent but no response: Quark issue
```

### Issue: Navigation Not Working (Specific)

**Symptoms:**
- Cmd+click does nothing
- "Go to Definition" from menu does nothing
- NO requests in stdin log

**Likely Cause:** Config fields breaking navigation (see ADR-002)

**Fix:**
```bash
# 1. Check config.toml
cat languages/SuperCollider/config.toml

# 2. Remove these fields if present:
# opt_into_language_servers = [...]
# scope_opt_in_language_servers = [...]

# 3. Should have ONLY:
# name, grammar, path_suffixes, line_comments, tab_size, hard_tabs

# 4. Reload extension
# Cmd+Shift+P → "zed: reload extensions"

# 5. Verify requests now sent
grep -i "definition" /tmp/sc_launcher_stdin.log
```

See `.ai/decisions/002-config-fields.md` for full details.

### Issue: Hover Not Working

**Symptoms:**
- Hovering over symbols shows nothing or generic info

**Expected Behavior:** This is normal - LanguageServer.quark doesn't implement `textDocument/hover`

**Not a bug.** Hover docs not available in current Quark version.

### Issue: "Unhandled notification supercollider/serverStatus"

**Symptoms:**
- Warning in Zed logs
- Features still work

**Expected Behavior:** This is a known non-blocking issue.

**Explanation:** Custom notification from Quark that Zed doesn't recognize. Doesn't affect functionality.

**Fix:** Can be safely ignored, or remove notification from Quark if desired.

## After Making Fixes

### Clean Restart Procedure

```bash
# 1. Kill all processes
pkill -9 sc_launcher
pkill -9 sclang

# 2. Wait a moment
sleep 1

# 3. Clear logs
rm -f /tmp/sc_launcher_stdin.log
rm -f /tmp/sclang_post.log

# 4. Restart Zed or reload extensions
# Cmd+Shift+P → "zed: reload extensions"

# 5. Reopen .scd file

# 6. Verify startup
tail -f /tmp/sclang_post.log
# Look for "Found providers" line
```

### Re-run Diagnostics

After fixes, verify with:

```bash
# LSP active?
grep -c "textDocument/" /tmp/sc_launcher_stdin.log
# Should be > 0

# Sclang clean?
grep -c "error" /tmp/sclang_post.log
# Should be 0 or very low

# Navigation working?
grep -c "definition" /tmp/sc_launcher_stdin.log
# Try cmd+click, should increment
```

## If Still Broken

1. **Check for similar past issues:**
   ```bash
   ls .ai/research/
   # Read relevant research docs
   ```

2. **Compare working vs broken:**
   - Try feature in Rust extension (known working)
   - Compare log output
   - Identify differences

3. **Test with minimal file:**
   - Create simple .scd: `"hello".postln`
   - Isolate whether issue is file-specific

4. **Check Zed version:**
   ```bash
   # In Zed: About → Version
   # Some features may require specific Zed versions
   ```

5. **Create research doc:**
   - Document symptoms, investigation steps
   - Save findings to `.ai/research/YYYY-MM-DD-issue-name.md`
   - Include log excerpts as evidence

## Useful References

- `.ai/context.md` - Anti-patterns and known issues
- `.ai/architecture.md` - System design, component interactions
- `.ai/decisions/002-config-fields.md` - Config issues that break navigation
- `.ai/commands.md` - All verification commands
