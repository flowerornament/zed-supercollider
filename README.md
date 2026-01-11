# SuperCollider Extension for Zed

> **Alpha Software** - This extension is under active development. Expect bugs, breaking changes, and rough edges. For adventurous users who want to help shape the project.

Zed extension for [SuperCollider](https://supercollider.github.io/) with LSP support and HTTP-based code evaluation.

## Features

- **Language Server Protocol**: Go-to-definition, find references, hover documentation, completions
- **Code Evaluation**: Play buttons on code blocks, fire-and-forget eval via HTTP
- **Server Control**: Boot, stop, recompile, quit via Zed tasks
- **Syntax Highlighting**: Tree-sitter grammar with SuperCollider-specific queries

## Prerequisites

- **macOS only** (Windows/Linux not yet supported)
- **SuperCollider** installed ([download](https://supercollider.github.io/downloads))
- **Zed** editor ([download](https://zed.dev/))
- **Rust** via [rustup](https://rustup.rs/) (required for building dev extensions)
  - Homebrew-installed Rust will not work with Zed dev extensions

## Installation (Development Mode)

This is the current installation method. Distribution will simplify in the future.

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

This creates `server/launcher/target/release/sc_launcher`.

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

### 6. Install tasks (optional but recommended)

Copy the task definitions for code evaluation and server control:

```bash
scripts/install-tasks.sh
```

Or manually copy `.zed/tasks.json` to your SuperCollider workspace.

## Usage

### Opening SuperCollider files

Open any `.sc` or `.scd` file. The extension activates automatically.

### Code evaluation

- Click the **play button** on parenthesized blocks `(...)` or function blocks `{...}`
- Results appear in the Post Window (see below)

### Server control

Use Zed tasks (`Cmd+Shift+R`) for:
- **SC: Boot Server** - Start the audio server
- **SC: Stop (CmdPeriod)** - Stop all sounds
- **SC: Recompile** - Recompile the class library
- **SC: Quit Server** - Quit the audio server
- **SC: Post Window** - Tail the output log

### Key bindings

Two keymap files are provided. Copy your preferred one to `~/.config/zed/keymap.json`.

#### SC IDE Style (`.zed/keymap.json`)

Matches [SC IDE defaults](https://doc.sccode.org/Reference/KeyboardShortcuts.html). Best for users coming from SuperCollider IDE.

| Function | Shortcut | SC IDE |
|----------|----------|--------|
| Evaluate | `Cmd+Return`, `Shift+Return` | ✓ |
| Stop | `Cmd+.` | ✓ |
| Boot Server | `Cmd+B` | ✓ |
| Recompile | `Cmd+K` | ✓ |
| Help | `Cmd+D` | ✓ |

**Note:** These override some Zed defaults (e.g., `Cmd+.` for CodeAction menu, `Cmd+D` for add selection).

#### VS Code Compatible (`.zed/keymap-vscode-compatible.json`)

Uses consistent `Cmd+Shift` prefix to avoid conflicts with VS Code/Cursor/Zed defaults.

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
| `SC_LAUNCHER_DEBUG` | Enable verbose logging | unset |

## Troubleshooting

### Validate configuration

```bash
scripts/validate-config.sh
```

### Check logs

Post window output: `${SC_TMP_DIR:-$TMPDIR}/sclang_post.log`

```bash
tail -f /tmp/sclang_post.log
```

### Test the HTTP API

```bash
curl -X POST -d "1 + 1" http://127.0.0.1:57130/eval
curl http://127.0.0.1:57130/health
```

### Common issues

- **Extension won't compile**: Ensure Rust is installed via rustup, not homebrew
- **LSP not starting**: Check that the launcher path in settings is correct and absolute
- **No play buttons**: Ensure `.zed/tasks.json` is present in your workspace

## Future

The current installation method requires building from source. Distribution will change to make installation simpler (e.g., direct installation from Zed's extension gallery).

## Contributing

Developer documentation lives in `.ai/` (start with `.ai/context.md`).

## License

[MIT](LICENSE)
