# Troubleshooting

Common issues and resolutions when setting up SuperCollider with Zed.

- sclang not found
  - Set `supercollider.sclangPath` to the absolute path. On Windows, point to `sclang.exe`.
- LanguageServer.quark missing
  - Install via the SuperCollider IDE (`Quarks.install("LanguageServer")`) and ensure the quark (plus `Log`, `UnitTest2`, `Deferred`) resides under `~/Library/Application Support/SuperCollider/{downloaded-quarks,Extensions}`.
  - Restart Zed after installing so the launcher picks up the quark.
- Play buttons missing
  - Ensure the document is saved with `.sc`/`.scd` and the cursor is inside a runnable block (`(...)` or `{...}`).
  - Confirm your tasks include `"tags": ["sc-eval"]` and use `$ZED_CUSTOM_code`.
- Eval task fails / connection refused
  - Make sure the launcher is running with the HTTP server enabled and the port matches your task (default `57130`).
  - Verify `curl` is available or use a launcher CLI fallback if configured.
- Server fails to boot / no sound
  - Check audio device configuration in your SuperCollider setup. Try `s.boot` manually, then `s.quit`.
- CmdPeriod doesn’t stop sound
  - Ensure the `/stop` task is hitting the launcher; check the terminal panel for errors.
- Conflicts with SCIDE
  - Avoid running SCIDE and Zed against the same `sclang_conf.yaml` simultaneously; use a dedicated config if needed.
- Help rendering blank
  - Set `supercollider.help.converter` (e.g., `pandoc`) or rely on LSP hover/docs.
- Ports in use
  - Check for existing `scsynth` instances; change ports in your SC config if needed.
  - The launcher uses two localhost UDP ports for LSP and an HTTP port (default `57130`) for evaluation.
