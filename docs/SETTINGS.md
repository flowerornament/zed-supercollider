# Settings

Configuration for the SuperCollider extension.

## Requirements

1. Install `LanguageServer.quark` via SuperCollider:
   ```supercollider
   Quarks.install("LanguageServer");
   ```
   This also installs dependencies: `Log`, `UnitTest2`, `Deferred`.

2. Ensure `sclang` is accessible (typically `/Applications/SuperCollider.app/Contents/MacOS/sclang` on macOS).

## LSP Configuration

Add to your Zed settings (`~/.config/zed/settings.json`):

```json
{
  "lsp": {
    "supercollider": {
      "binary": {
        "path": "/path/to/zed-supercollider/server/launcher/target/release/sc_launcher",
        "arguments": [
          "--mode", "lsp",
          "--http-port", "57130"
        ]
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

### LSP Settings Reference

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `languageServerLogLevel` | string | `"error"` | LanguageServer.quark verbosity |
| `sclang.postEvaluateResults` | string | `"true"` | Post evaluation results |
| `sclang.improvedErrorReports` | string | `"false"` | Enhanced backtraces |
| `sclang.evaluateResultPrefix` | string | `"> "` | Prefix for results |

## Tasks Configuration

Add to `.zed/tasks.json` in your project (or global tasks):

```json
[
  {
    "label": "SuperCollider: Evaluate",
    "command": "/path/to/zed-supercollider/.zed/eval.sh",
    "tags": ["sc-eval"],
    "reveal": "never",
    "hide": "always"
  },
  {
    "label": "SuperCollider: Stop (CmdPeriod)",
    "command": "curl",
    "args": ["-s", "-X", "POST", "http://127.0.0.1:57130/stop"],
    "reveal": "never",
    "hide": "always"
  },
  {
    "label": "SuperCollider: Boot Server",
    "command": "curl",
    "args": ["-s", "-X", "POST", "http://127.0.0.1:57130/boot"],
    "reveal": "never",
    "hide": "always"
  },
  {
    "label": "SuperCollider: Recompile",
    "command": "curl",
    "args": ["-s", "-X", "POST", "http://127.0.0.1:57130/recompile"],
    "reveal": "never",
    "hide": "always"
  },
  {
    "label": "SuperCollider: Quit Server",
    "command": "curl",
    "args": ["-s", "-X", "POST", "http://127.0.0.1:57130/quit"],
    "reveal": "never",
    "hide": "always"
  },
  {
    "label": "SuperCollider: Post Window",
    "command": "tail",
    "args": ["-f", "/tmp/sclang_post.log"],
    "reveal": "always",
    "use_new_terminal": true,
    "hide": "never"
  },
  {
    "label": "SuperCollider: Kill All (Emergency Cleanup)",
    "command": "sh",
    "args": ["-c", "pkill -9 sclang; pkill -9 scsynth; pkill -9 sc_launcher; rm -f /tmp/sclang_post.log; echo 'Killed all SuperCollider processes'"],
    "reveal": "always"
  }
]
```

### HTTP Endpoints

The launcher exposes these endpoints on port 57130:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/eval` | POST | Evaluate code (body = source) |
| `/stop` | POST | Stop all sounds (CmdPeriod) |
| `/boot` | POST | Boot audio server |
| `/quit` | POST | Quit audio server |
| `/recompile` | POST | Recompile class library |
| `/health` | GET | Health check |

### Eval Script

The evaluate task uses a wrapper script (`.zed/eval.sh`):

```bash
#!/bin/bash
if [ -z "$ZED_CUSTOM_code" ]; then
    exit 1
fi
curl -s -X POST \
    -H "Content-Type: text/plain" \
    --data-binary "$ZED_CUSTOM_code" \
    http://127.0.0.1:57130/eval > /dev/null 2>&1
```

Make it executable: `chmod +x .zed/eval.sh`
