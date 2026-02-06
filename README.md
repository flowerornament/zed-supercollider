# SuperCollider Extension for Zed

> **Early Release** - This extension is under active development. Core features work well, but expect some rough edges. Feedback welcome!

Zed extension for [SuperCollider](https://supercollider.github.io/) with LSP support and HTTP-based code evaluation.

## Features

- **Language Server Protocol**: Go-to-definition, find references, hover documentation, completions
- **Code Evaluation**: Play buttons, keyboard shortcuts, and CodeActions menu
- **Server Control**: Boot, stop, recompile, quit via tasks or CodeActions
- **Syntax Highlighting**: Tree-sitter grammar with SuperCollider-specific queries

## Prerequisites

- **macOS only** (Windows/Linux not yet supported)
- **SuperCollider** installed ([download](https://supercollider.github.io/downloads))
- **Zed** editor ([download](https://zed.dev/))
- **Rust** via [rustup](https://rustup.rs/)
  - Homebrew Rust won't work — Zed compiles extensions to WebAssembly, which requires the `wasm32-wasip1` target that only rustup can easily provide

## Installation

This extension isn't in the Zed extension gallery yet, so you'll build it from source.

### 1. Clone the repository

```bash
git clone https://github.com/flowerornament/zed-supercollider.git
cd zed-supercollider
```

### 2. Install the Rust WASM target

```bash
rustup target add wasm32-wasip1
```

### 3. Build the launcher

```bash
cd server/launcher
cargo build --release
cd ../..
```

### 4. Load the extension in Zed

1. Open Zed
2. Open the command palette (`Cmd+Shift+P`)
3. Run `zed: install dev extension`
4. Select the `zed-supercollider` directory

The extension will compile and load. You'll see "SuperCollider" in your extensions list.

### 5. Configure Zed settings

Open Zed settings (`Cmd+,`) and add:

```json
{
  "lsp": {
    "supercollider": {
      "binary": {
        "path": "/absolute/path/to/zed-supercollider/server/launcher/target/release/sc_launcher",
        "arguments": ["--mode", "lsp", "--http-port", "57130"]
      }
    }
  }
}
```

Replace `/absolute/path/to/zed-supercollider` with your actual clone location.

### 6. Verify installation

1. Open any `.sc` or `.scd` file in the cloned repo (e.g., `examples/` if present, or create a test file)
2. Run `SC: Check Setup` from tasks (`Cmd+Shift+R` → search "check")
3. You should see a play button (▶) appear on parenthesized blocks like `(1 + 1)`

## Usage

### Working from the repo

**Important:** Work on SuperCollider files from within the cloned `zed-supercollider` directory. The tasks and keymaps are already configured there.

To use SuperCollider in other projects, copy `.zed/tasks.json` and optionally `.zed/keymap.json` to that project's `.zed/` folder. The LSP settings (with the absolute path to `sc_launcher`) are in your global Zed config, so they'll work anywhere.

### Post Window

SuperCollider output (print statements, errors, server messages) goes to the Post Window. Open it with:
- Task: `SC: Post Window` (runs `tail -f` on the log file)
- Or directly: `tail -f /tmp/sclang_post.log`

### Tasks menu

Most SuperCollider commands are available in the **tasks menu**:
- `Cmd+Shift+R` (or `Cmd+Shift+P` → "task: spawn")

This is separate from the CodeActions menu (`Cmd+.`), which shows context-sensitive actions for the code under your cursor.

### Code evaluation

Multiple ways to evaluate code:

| Method | How |
|--------|-----|
| **Play button** | Click ▶ on parenthesized blocks `(...)` |
| **Keyboard** | `Cmd+Return` or `Shift+Return` (SC IDE style) |
| **Tasks menu** | `Cmd+Shift+R` → SC: Evaluate Selection/Line/Block |
| **CodeActions** | `Cmd+.` → SC: Evaluate Selection/Line/Block |

Play buttons appear on parenthesized code blocks detected by the tree-sitter grammar.

### Server control

Available via tasks (`Cmd+Shift+R`) or CodeActions (`Cmd+.`):

- **SC: Boot Server** - Start the audio server
- **SC: Stop (CmdPeriod)** - Stop all sounds
- **SC: Recompile** - Recompile the class library
- **SC: Quit Server** - Quit the audio server
- **SC: Post Window** - View output log
- **SC: Check Setup** - Diagnose configuration issues

### CodeActions menu

Press `Cmd+.` (or right-click) for context-sensitive actions:
- Evaluate Selection/Line/Block
- Server control (Boot, Stop, Recompile, Quit)
- Help for the class under cursor

### Key bindings

Three keymap files are provided in `.zed/`. Copy your preferred one to `~/.config/zed/keymap.json`.

#### SC IDE Style (`.zed/keymap.json`)

Best for users coming from SuperCollider IDE. Matches [SC IDE defaults](https://doc.sccode.org/Reference/KeyboardShortcuts.html).

| Function | Shortcut |
|----------|----------|
| Evaluate | `Cmd+Return`, `Shift+Return` |
| Stop | `Cmd+.` |
| Boot Server | `Cmd+B` |
| Recompile | `Cmd+K` |
| Help | `Cmd+D` |

**Note:** Overrides some Zed defaults (`Cmd+.`, `Cmd+D`, `Cmd+B`).

#### Zed-Native (`.zed/keymap-zed-native.json`) — Recommended

Uses `Ctrl` prefix to avoid ALL conflicts with Zed defaults. Best for users who want SC alongside normal Zed shortcuts.

| Function | Shortcut |
|----------|----------|
| Evaluate | `Ctrl+Return` |
| Stop | `Ctrl+.` |
| Boot Server | `Ctrl+B` |
| Quit Server | `Ctrl+Q` |
| Recompile | `Ctrl+R` |
| Help | `Ctrl+H` |

#### VS Code Compatible (`.zed/keymap-vscode-compatible.json`)

Uses `Cmd+Shift` prefix to avoid conflicts with standard editor shortcuts.

| Function | Shortcut |
|----------|----------|
| Evaluate | `Cmd+Shift+Return` |
| Stop | `Escape` |
| Boot Server | `Cmd+Shift+B` |
| Quit Server | `Cmd+Shift+Q` |
| Recompile | `Cmd+Shift+R` |
| Help | `Cmd+Shift+D` |

### LSP features

- **Hover** over symbols for documentation
- **Go to Definition** (`Cmd+Click` or `F12`)
- **Find References** (`Shift+F12`)
- **Completions** as you type

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `SC_HTTP_PORT` | HTTP port for eval/control | `57130` |
| `SC_TMP_DIR` | Log file directory | `$TMPDIR` or `/tmp` |
| `RUST_LOG` | Log level (e.g. `sc_launcher=debug`) | `sc_launcher=info` |

## Troubleshooting

### Run the setup checker

```bash
scripts/check-setup.sh
```

This validates your configuration and shows diagnostic info.

### Validate extension config

```bash
scripts/validate-config.sh
```

### Common issues

- **Extension won't compile**: Ensure Rust is installed via [rustup](https://rustup.rs/), not Homebrew. Zed needs the `wasm32-wasip1` target.
- **LSP not starting**: Check that the launcher path in Zed settings is correct and absolute.
- **Play buttons appear but don't work**: The tasks aren't loaded. Make sure you're working from the cloned repo, or copy `.zed/tasks.json` to your project.
- **No output visible**: Open the Post Window (`SC: Post Window` task) to see SuperCollider output.

### Check logs

Post window output:
```bash
tail -f /tmp/sclang_post.log
```

### Test the HTTP API

```bash
curl -X POST -d "1 + 1" http://127.0.0.1:57130/eval
curl http://127.0.0.1:57130/health
```

## Future

Distribution will simplify — the goal is direct installation from Zed's extension gallery without building from source.

## Contributing

Developer documentation lives in `.agents/` (start with `.agents/architecture.md`).

## License

[MIT](LICENSE)
