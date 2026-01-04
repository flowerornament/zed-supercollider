# SuperCollider for Zed

SuperCollider language support for the [Zed](https://zed.dev) editor.

## Features

- **Syntax highlighting** via Tree-sitter grammar
- **Code evaluation** with inline play buttons (Code Lens)
- **LSP support** for completions, go-to-definition, find references
- **Post Window** for sclang output
- **Audio server control** (boot, stop, quit)

## Requirements

- [Zed](https://zed.dev) editor
- [SuperCollider](https://supercollider.github.io/) 3.12+
- [LanguageServer.quark](https://github.com/scztt/LanguageServer.quark)

## Installation

### 1. Install LanguageServer.quark

In SuperCollider IDE:
```supercollider
Quarks.install("LanguageServer");
// Recompile class library after installing
```

### 2. Build the launcher

```bash
cd server/launcher
cargo build --release
```

### 3. Configure Zed

Add to your Zed settings (`~/.config/zed/settings.json`):

```json
{
  "lsp": {
    "supercollider": {
      "binary": {
        "path": "/path/to/zed-supercollider/server/launcher/target/release/sc_launcher",
        "arguments": ["--mode", "lsp"]
      }
    }
  }
}
```

### 4. Add keybindings

Add to `~/.config/zed/keymap.json`:

```json
[
  {
    "context": "Editor && extension == scd",
    "bindings": {
      "cmd-enter": ["task::Spawn", { "task_name": "SuperCollider: Evaluate" }],
      "cmd-.": ["task::Spawn", { "task_name": "SuperCollider: Stop (CmdPeriod)" }],
      "cmd-shift-b": ["task::Spawn", { "task_name": "SuperCollider: Boot Server" }],
      "ctrl-shift-p": ["task::Spawn", { "task_name": "SuperCollider: Post Window" }]
    }
  }
]
```

### 5. Set up tasks

Copy `.zed/tasks.json` to your project or see [docs/SETTINGS.md](docs/SETTINGS.md) for task configuration.

## Quick Start

1. Open a `.scd` file
2. Press `ctrl-shift-p` to open the Post Window
3. Press `cmd-shift-b` to boot the audio server
4. Click play buttons in the gutter or press `cmd-enter` to evaluate code
5. Press `cmd-.` to stop all sounds

## Documentation

- [Usage Guide](docs/USAGE.md) - Day-to-day workflow and keybindings
- [Settings](docs/SETTINGS.md) - LSP and task configuration
- [Troubleshooting](docs/TROUBLESHOOTING.md) - Common issues and fixes
- [Contributing](docs/CONTRIBUTING.md) - Development setup

## LSP Features

| Feature | Status |
|---------|--------|
| Go to Definition | Working |
| Find References | Working |
| Completions | Working (on `.`, `(`, `~`) |
| Signature Help | Working |
| Document Symbols | Working |
| Code Lens (play buttons) | Working |
| Hover Documentation | Not available |

## License

MIT License - see [LICENSE](LICENSE) for details.
