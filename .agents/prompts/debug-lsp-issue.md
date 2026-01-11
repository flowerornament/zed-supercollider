title: "Prompt: Debug LSP Issue"
created: 2026-01-05
updated: 2026-01-06
purpose: "Minimal checklist to triage LSP feature failures fast"
---

# Prompt: Debug LSP Issue

Use this when navigation/completion/hover stop working.

## Quick triage (run in order)
```bash
tail -20 ~/Library/Logs/Zed/Zed.log | grep -i "supercollider\\|language server"   # did the client start it?
tail -20 /tmp/sc_launcher_stdin.log                                               # any LSP traffic?
grep -i "error\\|exception\\|dnu" /tmp/sclang_post.log | tail -10                 # quark errors?
```

## If LSP never started
- Confirm launcher exists: `ls server/launcher/target/release/sc_launcher`.
- Check Zed settings path/args; rebuild launcher if missing (`cd server/launcher && cargo build --release`).
- Manual start to see errors: `./server/launcher/target/release/sc_launcher --mode lsp`.

## If no requests beyond initialize
- Re-check `languages/SuperCollider/config.toml` for banned fields (`opt_into_language_servers`, `scope_opt_in_language_servers`); remove and reload extension.
- Look for connection errors in `~/Library/Logs/Zed/Zed.log`.

## If requests exist but no responses/features
- Inspect `/tmp/sclang_post.log` for DNUs or provider errors.
- Ensure installed LanguageServer matches vendored copy if errors persist.
- Verify capabilities advertised: `grep -A 80 '"capabilities"' /tmp/sc_launcher_stdin.log`. Current launcher still reports extra providers (signature/folding/selection/workspaceSymbol/codeLens); trim to definition/references/completion/hover/executeCommand once capability hygiene is fixed.

## Clean restart loop
```bash
pkill -9 sc_launcher sclang
rm -f /tmp/sc_launcher_stdin.log /tmp/sclang_post.log
# reload extension / reopen .scd
```

## Evidence to record (for research docs)
- Commands run + key log excerpts.
- Whether `textDocument/definition` appears after reload.
- Any quark error strings (DNUs, Non Boolean in test, etc.).
