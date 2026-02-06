title: "Architecture"
created: 2026-01-05
updated: 2026-01-11
purpose: "Concise mental model and extension points for the dual-channel SuperCollider Zed extension"
---

# Architecture

3-piece bridge: **Zed Extension (WASM)** → **Launcher (Rust)** → **sclang/LanguageServer.quark (UDP)**. LSP flows over stdio↔UDP; eval/control flow over HTTP because the extension API cannot trigger `workspace/executeCommand`.

## System diagram
```
┌──────────────────────────────────────────────────────────────┐
│ Zed Editor                                                   │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│ runnables.scm → play buttons → tasks → HTTP POST             │
│                                           │                  │
│ LSP client ─────────────── stdio ─────────┼────────┐         │
│                                           │        │         │
└───────────────────────────────────────────┼────────┼─────────┘
                                            │        │
                                            ▼        ▼
┌──────────────────────────────────────────────────────────────┐
│ sc_launcher (Rust)                                           │
│  ┌──────────────────┐  ┌────────────────────────────────┐   │
│  │ HTTP server      │  │ LSP bridge                     │   │
│  │ :57130           │  │ stdio ↔ UDP translation        │   │
│  │ /eval /stop      │  │ buffer until "***LSP READY***" │   │
│  │ /boot /recompile │  │ chunk >8KB UDP                 │   │
│  │ /quit /health    │  └───────────┬────────────────────┘   │
│  └────────┬─────────┘              │                         │
│           │                        │                         │
│           └────────────┬───────────┘                         │
│                        ▼                                     │
│                   UDP socket (localhost)                     │
└────────────────────────┼────────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────────┐
│ sclang --daemon                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ LanguageServer.quark                                   │ │
│  │  - providers: definition/references/completion/hover   │ │
│  │    /executeCommand                                     │ │
│  │  - TextDocumentProvider queues early msgs, rehydrates   │ │
│  │  - LSPDatabase indexing                                 │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│ → stdout/stderr → /tmp/sclang_post.log                       │
└──────────────────────────────────────────────────────────────┘
```

## System at a glance
- Extension (`src/lib.rs`): chooses launcher path, merges settings, passes LSP bytes through.
- Launcher (`server/launcher/src/main.rs`): stdio↔UDP bridge with buffering and chunking, HTTP server on `:57130` (`/eval`, `/stop`, `/boot`, `/recompile`, `/quit`, `/health`), spawns `sclang --daemon`, forwards stderr to stdout/logs; HTTP replies 202 `{"status":"sent"}` and currently allows CORS `*` with no body size guard.
- Quark (`server/quark/LanguageServer.quark/`): provider pattern for definition/references/completion/hover/executeCommand. `TextDocumentProvider` queues early events and rehydrates docs.
- Runnables/tasks: `runnables.scm` tags code blocks; `.zed/tasks.json` POSTs tagged blocks to HTTP endpoints.
- Capabilities advertised in initialize: signatureHelp, selectionRange, foldingRange, codeLens, workspaceSymbol, declaration/implementation, and the working set (definition/references/completion/hover/executeCommand). Reliable support is the working set; signatureHelp unverified; document symbols are never requested by the client.

## Key flows
- **Navigation (LSP):** Zed → stdio → launcher → UDP → quark providers → UDP → stdout → Zed. Logs: `/tmp/sc_launcher_stdin.log` (requests/responses), `/tmp/sclang_post.log` (provider errors).
- **Eval/control (HTTP):** Play button → task → `curl http://127.0.0.1:57130/<endpoint>` → launcher → UDP → quark `ExecuteCommandProvider`. Result appears in `/tmp/sclang_post.log`/Post Window.
- **Readiness:** Launcher buffers until it sees `"***LSP READY***"` from the quark, then flushes pending messages.
- **Chunking:** UDP payloads chunked (~6KB) to avoid MTU issues; oversized HTTP bodies should be rejected or chunked.

## Why two channels (LSP + HTTP)
- Zed extension API cannot programmatically send `workspace/executeCommand`, so eval/control cannot ride the LSP channel.
- Tasks + HTTP give us play buttons, keybindings, and direct `curl` for debugging while keeping LSP for intelligence.
- Separation keeps eval/control tolerant of client quirks and debuggable even when LSP misbehaves.

## Extension points (lightweight)
- Add LSP capability: implement/register provider in quark → advertise in launcher initialize response → no extension change.
- Add HTTP endpoint: new handler in launcher HTTP section → new command in `ExecuteCommandProvider` → add matching task/tag.
- Add runnable: add tree-sitter capture/tag in `runnables.scm` → add task with same tag.

## Failure modes to check first
- No navigation: config fields wrong (`opt_into_language_servers`), launcher missing, or initialize not flushed. Check `/tmp/sc_launcher_stdin.log` for definition requests.
- Eval dead: launcher not running or port blocked. `curl -X POST -d "1+1" http://127.0.0.1:57130/eval`.
- Quark crash/no responses: look for errors/DNU in `/tmp/sclang_post.log`; restart `sclang` and ensure installed quark matches vendored copy.
