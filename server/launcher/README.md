# SuperCollider LSP Launcher

Responsibilities
- Detect `sclang` (or use `--sclang-path`).
- Warn if `LanguageServer.quark` is missing.
- Start `sclang --daemon` with the LanguageServer environment.
- Bridge JSON-RPC between LSP stdio and the Quark’s UDP transport.

Usage
- Probe `sclang` availability:
  - `sc_launcher --mode probe --sclang-path /path/to/sclang`
- Run the LSP bridge (what Zed calls):
  - `sc_launcher --mode lsp --sclang-path /path/to/sclang`
  - Optional: `--conf-yaml-path /path/to/sclang_conf.yaml`
  - Optional: `--log-level info` (forwarded to `SCLANG_LSP_LOGLEVEL`)

Implementation Notes
- Reserves two localhost UDP ports and exposes them via `SCLANG_LSP_CLIENTPORT` / `SCLANG_LSP_SERVERPORT`.
- Streams stdin → UDP and UDP → stdout while relaying `sclang` stdout/stderr to the launcher’s stderr.
- Shuts down `sclang` when the client closes stdin.
- Expects `LanguageServer.quark` (and its dependencies) to be installed in the user’s SuperCollider extensions/quarks directories.
