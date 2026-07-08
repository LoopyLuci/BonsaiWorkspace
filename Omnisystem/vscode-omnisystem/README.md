# Omnisystem Languages — VS Code Extension

Full IDE support for the **Omnisystem 7-language ecosystem** inside Visual Studio Code.

## Languages Supported

| Language | Extension | Purpose |
|----------|-----------|---------|
| **TITAN** | `.titan` | Systems language — Rust-like syntax, memory-safe, zero-cost abstractions |
| **VERA** | `.vera` | UI components — reactive props/state/render model |
| **HELIX** | `.helix` | GPU shaders — pipelines, vertex/fragment/compute stages |
| **AETHER** | `.aether` | Actor model concurrency — messages, handlers, spawn |
| **AXIOM** | `.axiom` | Formal verification — theorems, preconditions, postconditions |
| **SYLVA** | `.sylva` | Neural networks — layers, models, training |
| **NEXUS** | `.nexus` | Responsive layouts — flexbox/grid, breakpoints |

## Features

- **Syntax Highlighting** — Full TextMate grammars for all 7 languages with semantic color coding
- **Language Server (LSP)** — Diagnostics, completions, hover info, go-to-definition, find references
- **Code Snippets** — Production-ready snippets for common patterns in every language
- **Build Integration** — Run `omnicc build/run/test/clean` from inside VS Code
- **Task Definitions** — Integrated task runner with problem matchers for compiler errors
- **Debug Support** — Launch and attach configurations via LLDB
- **Status Bar** — Live LSP server status with one-click output access
- **Editor Menus** — Build/Run/Test buttons in the editor title bar for Omni-Language files

## Requirements

- VS Code 1.85.0 or later
- **OmniCC compiler** (`omnicc`) on your system PATH, or configured via `omnisystem.omniccPath`
- The Omnisystem LSP server (either the standalone `LspServer` binary or `omnicc lsp` subcommand)

## Getting Started

1. **Install the extension** from the VS Code Marketplace or by installing the `.vsix` package
2. **Open a workspace** that contains `.titan`, `.vera`, `.helix`, `.aether`, `.axiom`, `.sylva`, or `.nexus` files
3. The extension activates automatically when you open any Omni-Language file
4. **Build your project**: Press `Ctrl+Shift+B` (or use the toolbar button)
5. **Run your project**: Press `F5` or click the play button in the editor title bar

### Building the Extension Yourself

```bash
cd vscode-omnisystem
npm install
npm run compile
npm run package   # produces omnisystem-1.0.0.vsix
```

Install the resulting `.vsix`:
```bash
code --install-extension omnisystem-1.0.0.vsix
```

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `omnisystem.omniccPath` | `"omnicc"` | Path to the OmniCC compiler executable |
| `omnisystem.lspServerPath` | `""` | Path to the LSP server binary (auto-detected if empty) |
| `omnisystem.buildTarget` | `"x86_64-windows"` | Compilation target platform |
| `omnisystem.optimizationLevel` | `"O0"` | Compiler optimization level |
| `omnisystem.enableLsp` | `true` | Enable/disable the language server |
| `omnisystem.enableInlayHints` | `true` | Show/hide inlay type hints |

### Build Targets

| Target | Description |
|--------|-------------|
| `x86_64-linux` | Linux x86-64 |
| `x86_64-windows` | Windows x86-64 |
| `arm64-macos` | macOS Apple Silicon |
| `wasm` | WebAssembly |

### Optimization Levels

| Level | Description |
|-------|-------------|
| `O0` | No optimization — fastest compile, best debug info |
| `O1` | Basic optimizations |
| `O2` | Standard optimizations |
| `O3` | Aggressive optimization — slowest compile, fastest binary |

## Keyboard Shortcuts

| Shortcut | Command | When |
|----------|---------|------|
| `Ctrl+Shift+B` | Omnisystem: Build | Omni-Language file open |
| `F5` | Omnisystem: Run | Omni-Language file open, not debugging |

## Commands

All commands are available via `Ctrl+Shift+P` (Command Palette):

| Command | Description |
|---------|-------------|
| `Omnisystem: Build` | Compile the current project |
| `Omnisystem: Run` | Build and run the project |
| `Omnisystem: Test` | Run all tests |
| `Omnisystem: Clean` | Clean build artifacts |
| `Omnisystem: Restart Language Server` | Restart the LSP server |
| `Omnisystem: Show Output` | Open the Omnisystem output channel |

## LSP Server Setup

The extension attempts to find the LSP server in the following order:

1. The path set in `omnisystem.lspServerPath`
2. A compiled binary (`omnicc-lsp` or `LspServer`) next to the OmniCC executable
3. A compiled binary in `${workspaceFolder}/build/`
4. The OmniCC executable with the `lsp --stdio` subcommand

If OmniCC supports `omnicc lsp --stdio`, no additional configuration is needed.

## Problem Matcher

The extension registers a problem matcher for OmniCC error output in the format:

```
path/to/file.titan:12:5: error: undefined variable 'foo'
```

This format is compatible with the VS Code task infrastructure and shows errors inline in the editor.

## Language Snippets Reference

### TITAN
- `fn` — function definition
- `struct` / `pstruct` — struct (public)
- `enum` — enum definition
- `impl` / `implt` — impl block / impl trait
- `test` / `testmod` — test function / test module
- `match` — match expression
- `iflet` / `whilelet` — pattern matching
- `for` — for-in loop
- `vec` / `veclit` / `hashmap` — collections

### VERA
- `component` / `comp` — full or minimal component
- `props` / `state` — block declarations
- `onclick` / `onchange` / `onsubmit` — event handlers
- `usestate` / `useeffect` / `useref` — hooks

### HELIX
- `pipeline` — full render pipeline
- `vert` / `frag` / `compute` — individual shader stages
- `computepipeline` — compute-only pipeline
- `uniforms` — uniforms block
- `sample` — texture sample call

### AETHER
- `actor` / `actors` — actor (with optional state)
- `message` — message declaration
- `handler` — handler function
- `spawn` / `spawndo` — spawn actor
- `send` — send message

### AXIOM
- `theorem` / `thm` — theorem declaration
- `pre` / `post` / `inv` — block shorthands
- `forall` / `exists` — quantifiers
- `implies` / `iff` — logical operators
- `verify` / `prove` — verification calls

### SYLVA
- `layer` — generic layer
- `dense` — Dense layer
- `conv2d` — Conv2D layer
- `model` / `modeltrain` — model declarations
- `dropout` / `batchnorm` — regularization layers
- `adam` — optimizer config

### NEXUS
- `layout` — layout declaration
- `flex` / `flexcol` — flex containers
- `grid` — grid container
- `responsive` — full responsive layout
- `container` — centered container
- `breakpoint` — breakpoint rule

## Contributing

The extension source is in `vscode-omnisystem/src/extension.ts`. TextMate grammars are in `syntaxes/`, snippets in `snippets/`.

To contribute grammar improvements, follow the [TextMate grammar specification](https://macromates.com/manual/en/language_grammars).

## License

MIT — see the Omnisystem project root for full license text.
