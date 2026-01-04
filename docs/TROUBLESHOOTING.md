# Troubleshooting

Common issues and resolutions when using SuperCollider with Zed.

## Setup Issues

### sclang not found
Set the sclang path in your Zed settings. On macOS:
```
/Applications/SuperCollider.app/Contents/MacOS/sclang
```

### LanguageServer.quark missing
Install via SuperCollider:
```supercollider
Quarks.install("LanguageServer");
```
Restart Zed after installing.

### Language server not loading
1. Run `cmd-shift-p` → "zed: restart language servers"
2. Or close and reopen the `.scd` file
3. Or run "Kill All" and reopen

## Evaluation Issues

### Play buttons missing
- Ensure the file is saved with `.sc` or `.scd` extension
- Cursor must be inside a runnable block `(...)` or `{...}`
- Check that tasks include `"tags": ["sc-eval"]`

### Eval task fails / connection refused
- Verify the launcher is running (check LSP logs)
- Confirm HTTP port matches (default `57130`)
- Test with: `curl -X POST -d "1+1" http://127.0.0.1:57130/eval`

### No sound / server fails to boot
- Check audio device configuration in SuperCollider
- Try booting manually: evaluate `s.boot` in SuperCollider IDE first
- Check Post Window for error messages

### CmdPeriod doesn't stop sound
- Ensure `/stop` endpoint is accessible
- Check for orphaned scsynth processes (see below)

## Process Issues

### Orphaned scsynth processes
If sclang crashes, scsynth may keep running with no way to control it.

**Solution:** Use "SuperCollider: Kill All" (`cmd-alt-k`) to kill all SC processes.

### Multiple SuperCollider instances
The launcher kills existing sclang processes on startup. If you see duplicates:
1. Run "Kill All" task
2. Wait a moment
3. Reopen your `.scd` file

## LSP Issues

### Go to Definition not working
1. Ensure the word under cursor is a valid SuperCollider class or method name
2. Try `cmd-click` or `F12` on a class name like `SinOsc`
3. Check that LanguageServer.quark is installed and working
4. Look in Post Window for errors

### No hover documentation
**Status:** Not implemented. LanguageServer.quark does not provide `textDocument/hover`.

Hover documentation shows Zed's built-in word info, not SuperCollider docs.

### Completions not appearing
- Completions trigger on `.`, `(`, or `~`
- Type `SinOsc.` to see method completions
- Type `~` for environment variable completions
- Plain word completions (without trigger) are Zed's built-in feature, not LSP

### LSP not starting
Check the Zed logs for errors:
1. Open Command Palette → "zed: open log"
2. Search for "supercollider" or "sc_launcher"
3. Look for connection refused or path errors

To restart the LSP:
1. Command Palette → "zed: restart language servers"
2. Or close and reopen the `.scd` file

## Known Limitations

### Terminal flash when evaluating
When the terminal panel is open, evaluating code causes a brief flash as Zed creates/destroys terminals.

**Workaround:** Keep the terminal panel closed during normal coding. The Post Window can remain open.

**Status:** Zed limitation - tasks always create terminals, even with `"reveal": "never"`.

### Multiple Post Windows
Pressing `ctrl-shift-p` multiple times opens duplicate Post Window terminals.

**Workaround:** Only open the Post Window once per session.

**Status:** Zed tasks don't support singleton/toggle behavior.

### Post Window shows old content
The log file (`/tmp/sclang_post.log`) accumulates across sessions.

**Workaround:** Run "Kill All" to clear the log, or manually delete the file.

## Port Conflicts

The extension uses these ports:
- HTTP eval server: `57130` (configurable via `--http-port`)
- LSP UDP: dynamic localhost ports

If ports conflict, check for existing `scsynth` or `sclang` instances.

## Conflicts with SC IDE

Avoid running SC IDE and Zed simultaneously - they may conflict over `sclang_conf.yaml`.
