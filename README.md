# SuperCollider Zed Extension

Zed extension for SuperCollider with LSP support and HTTP-based code evaluation.

## Current State

Working: Navigation, code evaluation, server control

See `.ai/context.md` for details

## Setup

1) Install LanguageServer.quark in SuperCollider: `Quarks.install("LanguageServer");`
2) Build the launcher: `cd server/launcher && cargo build --release` (or place `sc_launcher` on PATH)
3) Configure Zed settings (sample):
```json
{
  "lsp": {
    "supercollider": {
      "binary": {
        "path": "/path/to/sc_launcher",
        "arguments": ["--mode", "lsp", "--http-port", "57130"]
      }
    }
  }
}
```
4) Optional: set `SC_HTTP_PORT` for task HTTP calls and `SC_TMP_DIR` for log location (defaults to TMPDIR).
   - For verbose stdout/stderr logging from the launcher, set `SC_LAUNCHER_DEBUG=1` (file logs remain gated by `SC_LAUNCHER_DEBUG_LOGS`).

## Usage

- Use the play button or `SuperCollider: Evaluate` task for code eval (HTTP, fire-and-forget; results appear in the Post Window).
- Control tasks (`Stop/Boot/Recompile/Quit`) hit `http://127.0.0.1:${SC_HTTP_PORT:-57130}`.
- Tail the Post Window via the `SuperCollider: Post Window` task (logs in `${SC_TMP_DIR:-${TMPDIR:-/tmp}}`).

## Troubleshooting

- Run `scripts/validate-config.sh` to enforce the minimal language config.
- Use the slash command `supercollider-check-setup` in Zed to probe launcher availability.
- Check `sclang_post.log` for runtime errors; log location respects `SC_TMP_DIR`/`TMPDIR`.

## Developer Docs

Contributor-facing notes live in `.ai/` (start with `.ai/context.md`).
