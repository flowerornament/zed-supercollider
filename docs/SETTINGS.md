# Extension Settings

Configured in Zed’s settings UI/JSON.

- supercollider.sclangPath (string)
  - Absolute path to `sclang`. If empty, PATH is used.
- supercollider.confYamlPath (string, optional)
  - Path to `sclang_conf.yaml`. Leave empty to use defaults.
- supercollider.postMode (string: "lsp" | "terminal")
  - Where to display post output; default "lsp".
- supercollider.autoBootServer (bool)
  - Auto-boot `s.boot` on first eval; default false.
- supercollider.help.converter (string, optional)
  - External converter for `.schelp` → Markdown (e.g., `pandoc`).

Commands (discoverable in Command Palette)
- SuperCollider: Check Setup — probes `sclang` availability and reports diagnostics.
- SuperCollider: Boot Server — calls `s.boot`.
- SuperCollider: Quit Server — calls `s.quit`.
- SuperCollider: Recompile Class Library — calls `thisProcess.recompile`.
- SuperCollider: Hard Stop — calls `CmdPeriod.run`.
- SuperCollider: Eval Line / Selection / Block — sends code to LSP.
