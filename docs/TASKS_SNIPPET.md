# Zed Task Snippet (optional post window fallback)

Add this to your Zed tasks to run a persistent `sclang` session in an integrated terminal.

Example (JSON):

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

Adjust `sclang` path as needed, or set environment variables as appropriate for your system.
