# Extension Settings

Configured in Zed’s settings UI/JSON.

- supercollider.languageServerLogLevel (string)
  - LanguageServer.quark verbosity; default `"error"`.
- supercollider.sclang.postEvaluateResults (string: "true" | "false")
  - Whether evaluated results are posted to the SuperCollider post buffer; default `"true"`.
- supercollider.sclang.improvedErrorReports (string: "true" | "false")
  - Enables enhanced backtraces; default `"false"`.
- supercollider.sclang.evaluateResultPrefix (string)
  - Prefix for evaluation results; default `"> "`.
- supercollider.sclang.guestEvaluateResultPrefix (string)
  - Result prefix when evaluating as a guest user; default `"[%|> "`.

LSP configuration in Zed settings
Add to project or user settings to point Zed at the launcher binary and arguments:

```
{
  "lsp": {
    "supercollider": {
      "binary": {
        "path": "/absolute/path/to/target/debug/sc_launcher",
        "arguments": [
          "--mode",
          "lsp",
          "--sclang-path",
          "/Applications/SuperCollider.app/Contents/MacOS/sclang",
          "--log-level",
          "info"
        ],
        "env": { }
      },
      "settings": {
        "supercollider": {
          "languageServerLogLevel": "info",
          "sclang": {
            "postEvaluateResults": "true",
            "improvedErrorReports": "false"
          }
        }
      }
    }
  }
}
```

Tasks and keybinding
Evaluation and server control run via Zed tasks that POST to the launcher's HTTP server. See
`docs/TASKS_SNIPPET.md` for examples.

Bind task invocations so evaluation and stop are one chord away:

```
{
  "keymaps": [
    {
      "context": "Editor && extension == scd",
      "bindings": {
        "ctrl-enter": ["task::Spawn", { "task_name": "SuperCollider: Evaluate" }],
        "ctrl-.": ["task::Spawn", { "task_name": "SuperCollider: Stop" }]
      }
    }
  ]
}
```

Requirements
- Install `LanguageServer.quark` (plus `Log`, `UnitTest2`, and `Deferred` dependencies) via SuperCollider’s Quarks UI or `Quarks.install("LanguageServer");`.
- Ensure `sclang` can access the quark in either `~/Library/Application Support/SuperCollider/downloaded-quarks/LanguageServer` or `~/Library/Application Support/SuperCollider/Extensions/LanguageServer`.
- Customize LSP behaviour by adding keys under `lsp.supercollider.settings.supercollider`. Omitted keys fall back to the defaults listed above.
