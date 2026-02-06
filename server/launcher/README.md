# SuperCollider LSP Launcher

A Rust bridge between Zed (or any LSP client) and SuperCollider's LanguageServer.quark. Translates LSP stdio to UDP and provides an HTTP server for code evaluation and control.

## Responsibilities

- Detect `sclang` (or use `--sclang-path`)
- Start `sclang --daemon` with the LanguageServer environment
- Bridge JSON-RPC between LSP stdio and the quark's UDP transport
- Provide HTTP API for eval/control (Zed can't call `workspace/executeCommand`)
- Clean up orphaned sclang processes on startup

## Quark Discovery

The launcher looks for [LanguageServer.quark](https://github.com/flowerornament/LanguageServer.quark) in this order:

1. **Vendored (preferred)**: `server/quark/LanguageServer.quark` relative to the repo root - used when developing or running from a repo checkout
2. **Installed**: User's SuperCollider quarks directory (`~/Library/Application Support/SuperCollider/downloaded-quarks/LanguageServer`)

When using the vendored quark, any installed version is excluded to avoid conflicts.

## Usage

**Probe sclang availability:**
```bash
sc_launcher --mode probe --sclang-path /path/to/sclang
```

**Run the LSP bridge (what Zed calls):**
```bash
sc_launcher --mode lsp --sclang-path /path/to/sclang
```

**Options:**
- `--conf-yaml-path /path/to/sclang_conf.yaml` - Custom sclang config
- `--log-level info` - LSP log level (error, warn, info, debug)
- `--http-port 57130` - HTTP server port (default: 57130)

## HTTP API

The launcher runs an HTTP server on port 57130 (configurable) for eval and control:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/eval` | POST | Execute SuperCollider code (body = code string) |
| `/health` | GET | Health check |
| `/stop` | POST | Stop all sounds (CmdPeriod) |
| `/boot` | POST | Boot the default server |
| `/recompile` | POST | Recompile class library |
| `/quit` | POST | Quit the server |
| `/convert-schelp` | POST | Convert .schelp to markdown (JSON body: `{"path": "..."}`) |

Example:
```bash
curl -X POST -d "1+1" http://127.0.0.1:57130/eval
```

## Implementation Notes

- Reserves two localhost UDP ports via `SCLANG_LSP_CLIENTPORT` / `SCLANG_LSP_SERVERPORT`
- Streams stdin → UDP and UDP → stdout
- Relays sclang stdout/stderr to launcher's stderr
- Shuts down sclang when the client closes stdin
- Buffers messages until `***LSP READY***` is received from the quark
- Chunks large UDP messages (>6KB) to avoid MTU issues
- Re-sends cached initialize/didOpen after class library recompile

## Environment Variables

- `RUST_LOG` - Log level filter (e.g. `RUST_LOG=sc_launcher=debug`)
- `SCLANG_LSP_LOGLEVEL` - Log level for the quark (set via `--log-level`)
- `SCHELP_LUA` - Path to schelp.lua for help conversion
