title: "Commands Reference"
created: 2026-01-05
updated: 2026-01-06
purpose: "Small set of commands that prove the extension works or is broken"
---

# Commands Reference

## Build/Reload
- Launcher: `cd server/launcher && cargo build --release` (output `server/launcher/target/release/sc_launcher`).
- Extension: in Zed → Cmd+Shift+P → `zed: reload extensions` (rebuilds WASM and reloads tree-sitter configs).
- Grammar changes: often require full Zed restart, not just reload extensions.

## Fast checks
- Eval request accepted: `curl -i -X POST -d "1 + 1" http://127.0.0.1:57130/eval` → expect HTTP 202 + `{"status":"sent","request_id":...}`; actual result appears in Post Window log (`sclang_post.log`).
- Navigation traffic: `grep -i "textDocument/definition" /tmp/sc_launcher_stdin.log`.
- Error scan: `grep -i "error\\|exception\\|dnu" /tmp/sclang_post.log`.
- Health ping: `curl http://127.0.0.1:57130/health`.

## Logs to watch
- LSP traffic: `tail -f /tmp/sc_launcher_stdin.log`.
- Eval/SC output: `tail -f /tmp/sclang_post.log`.
- Zed client: `tail -f ~/Library/Logs/Zed/Zed.log | grep -i supercollider`.

## Reset when things get weird
```bash
pkill -9 sc_launcher sclang scsynth
rm -f /tmp/sc_launcher_stdin.log /tmp/sclang_post.log
# reload extension or restart Zed, then reopen a .scd
```

## HTTP endpoints (manual)
```bash
curl http://127.0.0.1:57130/health
curl -X POST -d "1 + 1" http://127.0.0.1:57130/eval
curl -X POST http://127.0.0.1:57130/stop        # CmdPeriod
curl -X POST http://127.0.0.1:57130/boot
curl -X POST http://127.0.0.1:57130/recompile
curl -X POST http://127.0.0.1:57130/quit
```

## Tree-sitter / grammar testing
```bash
cd grammars/supercollider
tree-sitter generate                    # regenerate parser from grammar.js
tree-sitter parse ../../tests/test.scd  # verify parsing
tree-sitter query ../../languages/SuperCollider/runnables.scm ../../tests/test.scd  # test queries
```

## Troubleshooting quick hits
- LSP not starting: ensure launcher binary exists, check Zed settings path, try `./server/launcher/target/release/sc_launcher --mode lsp` to see errors.
- Navigation missing: re-check `languages/SuperCollider/config.toml` for banned fields; reload extension; look for `textDocument/definition` in `/tmp/sc_launcher_stdin.log`.
- Eval missing: confirm HTTP listening (`lsof -i :57130`), then POST as above.
- Installed quark mismatch: compare vendored files to `~/Library/Application Support/SuperCollider/downloaded-quarks/LanguageServer/` if errors persist.
