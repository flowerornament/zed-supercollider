title: "SuperCollider Zed Extension Context"
created: 2026-01-05
updated: 2026-01-06
purpose: "Slim brief for AI agents: what the project is, what works, what to avoid, and how to move fast without breaking users"
---

# SuperCollider Zed Extension

## What this project is
Ship a stable Zed extension for SuperCollider with navigation, completion, hover, and play-button evaluation. Architecture is intentionally dual-channel: LSP over stdio↔UDP for intelligence, HTTP for eval/control because Zed extensions cannot call `workspace/executeCommand` (see `.ai/decisions/001-http-not-lsp.md`).

## Current state (2026-01-07)
- Working: go-to-definition, hover (class doc block), completion, eval/control endpoints (HTTP returns 202 + request id; results land in Post log).
- Partial: references (built-ins like `MouseX` can still hit fallback issues).
- Capabilities advertised in initialize: signatureHelp, selectionRange, foldingRange, codeLens, workspaceSymbol, declaration/implementation, and the working set (definition/references/completion/hover/executeCommand). Reliable support is the working set; signatureHelp unverified; document symbols never requested by client.
- Logging defaults noisy: extension forces `SC_LAUNCHER_DEBUG`/`SC_LAUNCHER_DEBUG_LOGS`=1, so `/tmp` logs are on and stderr is chatty until we gate it.
- Known crash: “Non Boolean in test” in references provider when includeDeclaration is malformed; needs coercion.

## What to do first
1) Read `tasks/2026-01-05-execution-plan.md` (single backlog). Work P0 items before anything else.
2) Keep config and quark safety rules in mind (below) before changing code.
3) When you touch a task, bump `updated` + add a dated bullet to its `## Status Log`.

## Architecture snapshot
- Extension (`src/lib.rs`): picks launcher path (settings > PATH > dev build if it exists), merges settings, passes LSP stdio through untouched.
- Launcher (`server/launcher/src/main.rs`): stdio↔UDP bridge, HTTP server (`/eval`, `/stop`, `/boot`, `/recompile`, `/quit`), spawns/manages `sclang --daemon`, buffers until `***LSP READY***`, chunks UDP payloads.
- Quark (`server/quark/LanguageServer.quark/`): provider pattern for definition/references/completion/hover/executeCommand. `TextDocumentProvider` queues early events and rehydrates docs to survive startup races.
- Runnables/tasks (`languages/SuperCollider/runnables.scm`, `.zed/tasks.json`): play buttons tag code regions; tasks POST to launcher HTTP endpoints.

## Anti-patterns (do not regress)
- `languages/SuperCollider/config.toml`: keep only documented fields. Never add `opt_into_language_servers` or `scope_opt_in_language_servers`.
- SuperCollider dictionary functions: never use `^` (non-local return). It breaks `valueArray` return capture. Use expression returns instead.
- Dev launcher: only use local binary when it exists; otherwise honor settings/PATH. Do not `pkill` user sclang processes.
- Vendored quark: edit the copy in repo; avoid overwriting user-installed quark unless explicitly syncing for a test.

## Key files
- `src/lib.rs` – extension entry, launcher selection, settings merge.
- `server/launcher/src/main.rs` – LSP bridge + HTTP eval/control.
- `server/quark/LanguageServer.quark/` – LSP providers and database.
- `languages/SuperCollider/config.toml` – language config (stay minimal).
- `languages/SuperCollider/runnables.scm` – play-button captures.
- `.zed/tasks.json` – tasks that hit HTTP endpoints.
- `.ai/tasks/2026-01-05-execution-plan.md` – active backlog (P0–P2).

## Quick verification
- Build launcher: `cd server/launcher && cargo build --release`.
- Eval sanity: `curl -i -X POST -d "1 + 1" http://127.0.0.1:57130/eval` → expect HTTP 202 with `{"status":"sent","request_id":...}`; actual result appears in the Post Window log (`sclang_post.log`).
- Navigation traffic: `grep -i "textDocument/definition" /tmp/sc_launcher_stdin.log`.
- Error scan: `grep -i "error\\|exception\\|dnu" /tmp/sclang_post.log`.
- Clean restart: `pkill -9 sc_launcher sclang; rm -f /tmp/sc_launcher_stdin.log /tmp/sclang_post.log` then reload extension/open `.scd`.

## P0 focus (see task file for detail)
- Remove `serverStatus` noise and align advertised capabilities with what works.
- Safe dev launcher detection + clean shutdown (no orphaned or global kills).
- HTTP transport hardening (chunking/oversize handling, status codes, localhost-only).
- Logging defaults quiet; opt-in debug under TMP.
- Tasks/PID handling, probe JSON escaping, config validation/CI stub.

## Known limitations (expected)
- Hover/docs are minimal; outline/document symbols are absent because the client never requests them.
- Zed tasks briefly flash terminals; Post Window duplication is expected.
- Eval is fire-and-forget; results appear in `/tmp/sclang_post.log`/Post Window, not inline.

## Logs to watch
- `/tmp/sc_launcher_stdin.log` – LSP requests/responses.
- `/tmp/sclang_post.log` – eval output and sclang errors.
- `~/Library/Logs/Zed/Zed.log` – client-side info (search “supercollider”).
