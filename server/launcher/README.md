# SuperCollider LSP Launcher (stub)

Responsibilities
- Detect `sclang` (or use `--sclang-path`).
- Ensure `LanguageServer.quark` is installed (TBD).
- Start `sclang` with the LSP server and bridge it to stdio for Zed.

Usage (current stub)
- Probe sclang availability:
  - `sc_launcher --sclang-path /usr/local/bin/sclang` or just `sc_launcher` (uses PATH)

Next
- Add Quark install check and LSP bootstrap expression.
- Add stdio bridge for JSON‑RPC.
