# Troubleshooting

Common issues and resolutions when setting up SuperCollider with Zed.

- sclang not found
  - Set `supercollider.sclangPath` to the absolute path. On Windows, point to `sclang.exe`.
- Server fails to boot / no sound
  - Check audio device configuration in your SuperCollider setup. Try `s.boot` manually, then `s.quit`.
- CmdPeriod doesn’t stop sound
  - Ensure the LSP bridge receives the command; verify by watching the language server output panel.
- Conflicts with SCIDE
  - Avoid running SCIDE and Zed against the same `sclang_conf.yaml` simultaneously; use a dedicated config if needed.
- Help rendering blank
  - Set `supercollider.help.converter` (e.g., `pandoc`) or rely on LSP hover/docs.
- Ports in use
  - Check for existing `scsynth` instances; change ports in your SC config if needed.
