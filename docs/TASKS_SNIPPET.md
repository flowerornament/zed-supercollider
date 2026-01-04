# Zed Task Snippets

Primary eval tasks (runnables)

These tasks run when you click the gutter play button on a runnable block. They POST the
captured block (`$ZED_CUSTOM_code`) to the launcher's HTTP server.

```
{
  "tasks": [
    {
      "label": "SuperCollider: Evaluate",
      "command": "curl",
      "args": ["-s", "-X", "POST", "--data-binary", "$ZED_CUSTOM_code", "http://localhost:57130/eval"],
      "tags": ["sc-eval"],
      "hide": "on_success"
    },
    {
      "label": "SuperCollider: Stop",
      "command": "curl",
      "args": ["-s", "-X", "POST", "http://localhost:57130/stop"],
      "hide": "always"
    },
    {
      "label": "SuperCollider: Boot Server",
      "command": "curl",
      "args": ["-s", "-X", "POST", "http://localhost:57130/boot"],
      "hide": "on_success"
    }
  ]
}
```

Adjust the port if you changed it in the launcher. If `curl` is unavailable, use a launcher
CLI fallback if configured.

Optional post window fallback (persistent `sclang`)

Run a long-lived `sclang` session in an integrated terminal:

```
{
  "tasks": [
    {
      "label": "SC: Start sclang (post)",
      "command": "/usr/bin/env",
      "args": ["sclang"],
      "cwd": "${workspaceFolder}",
      "reveal": "always"
    }
  ]
}
```

Adjust `sclang` path and environment variables as needed for your system.
