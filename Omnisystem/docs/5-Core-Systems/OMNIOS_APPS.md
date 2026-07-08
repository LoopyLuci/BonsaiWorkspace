# OmniOS Desktop Applications — Complete Specifications

Every app in the OmniOS Desktop is a real, fully-functional application backed by the Runtime IPC layer. Nothing is simulated. Every click produces a real operation.

---

## App: Files (File Manager)

### Purpose
Browse, manage, and operate on the real workspace filesystem. The primary way users navigate their projects.

### Layout
```
┌─────────────────────────────────────────────────────────────┐
│ ← → ↑  [Breadcrumb: Projects / myapp / src]  [Search] [≡]  │
├──────────────┬──────────────────────────┬────────────────────┤
│ 📁 Projects  │  📂 src        dir       │ main.titan         │
│   📁 myapp   │  📄 main.titan 2.1 KB   │ ────────────────── │
│   📁 tests   │  📄 utils.titan 890 B   │ module Main;       │
│ 📁 Home      │  📄 BUILD.omni 512 B    │                    │
│ 📁 Packages  │  📂 tests      dir       │ fn main() -> i32 { │
│              │                          │   return 0;        │
│              │                          │ }                  │
└──────────────┴──────────────────────────┴────────────────────┘
│ 4 items · 3.5 KB · src/main.titan selected                   │
```

### Features
- **Left sidebar**: collapsible directory tree, persisted expanded state
- **Main area**: sorted listing (dirs first, then alpha) with name, size, modified date
- **Right preview pane**: syntax-highlighted file preview for all Omni-Language files; image preview for .png/.jpg; raw text for others
- **Navigation**: back/forward buttons (history stack), up button, breadcrumb clickable segments
- **Search**: real-time filter by filename; content search via `fs.search` IPC
- **File operations**: double-click to open in VS Code, right-click for context menu (Open, Rename, Copy, Move, Delete, Copy Path, Properties)
- **New file**: Ctrl+N → dialog with filename + type dropdown (pre-fills correct template for .titan, .vera, etc.)
- **Drag and drop**: drag files within the manager to move; drag to Code Studio to open for editing
- **Watch mode**: subscribes to `fs.watch` — directory listing updates live when files change externally
- **View modes**: large icons, detailed list (toggle with toolbar button)
- **Recent files**: right sidebar section showing last 20 opened files with timestamps

### IPC Commands Used
`fs.readDir`, `fs.readFile`, `fs.writeFile`, `fs.delete`, `fs.move`, `fs.search`, `fs.watch`, `fs.unwatch`

### Persistence
`~/.omnisystem/state/files.json`: `{lastPath, sidebarExpanded, viewMode, recentFiles[]}`

---

## App: Terminal (Real PTY)

### Purpose
A full terminal emulator providing direct access to the OS shell and the OmniOS runtime. Uses the same underlying technology as VS Code's integrated terminal.

### Layout
```
┌─────────────────────────────────────────────────────────────┐
│ [Terminal 1 ×] [Terminal 2 ×] [+]                   [⚙]   │
├─────────────────────────────────────────────────────────────┤
│ OmniOS Terminal v2.0                                        │
│ Type 'help' for OmniOS commands or use any shell command    │
│                                                             │
│ Z:\Projects\myapp> omnicc build                             │
│ [parse]   Scanning 8 source files...                        │
│ [resolve] Symbol resolution complete (142 symbols)          │
│ [type]    Type checking passed                              │
│ [codegen] Generating x86_64-windows target...               │
│ ✓ Build complete: target/x86_64-windows/myapp.exe (182 KB)  │
│                                                             │
│ Z:\Projects\myapp> _                                        │
└─────────────────────────────────────────────────────────────┘
```

### Features
- **Real PTY**: uses `node-pty` in the extension host, spawns actual shell (PowerShell on Windows, bash/zsh on Unix)
- **xterm.js renderer**: full VT100/xterm-256 support, font ligatures, selection, URL detection
- **Multiple tabs**: each tab is an independent PTY instance, Tab+Ctrl+T creates new tab
- **Scrollback buffer**: 10,000 lines per tab
- **Copy/paste**: Ctrl+Shift+C / Ctrl+Shift+V
- **Font size**: Ctrl+= to increase, Ctrl+- to decrease, Ctrl+0 to reset
- **Shell detection**: reads `$SHELL` environment variable; falls back to system default
- **OmniOS shell builtins**: `build`, `run`, `test`, `pm add`, `pm remove`, `fmt`, `check`, `doc`, `verify` are recognized and routed to the appropriate IPC command with live streaming output
- **Click to open file**: file paths in output (e.g., `src/main.titan:42`) are clickable, opens the file in VS Code at that line
- **Tab title**: shows current working directory, updates on `cd`
- **Split view**: button to split terminal horizontally (two PTYs side by side)

### IPC Commands Used
`term.create`, `term.input` (notification), `term.output` (notification stream), `term.resize`, `term.kill`

### Persistence
`~/.omnisystem/state/terminal.json`: `{tabs: [{id, cwd, shell, scrollbackLines}]}`

---

## App: Code Studio

### Purpose
The primary code creation interface. Scaffold new files and projects, run quick actions, navigate the codebase, and learn Omni-Languages.

### Layout
```
┌─────────────────────────────────────────────────────────────┐
│ ✨ Code Studio                                    [Expert ▾] │
├──────────────────┬──────────────────────────────────────────┤
│ NEW FILE         │ QUICK ACTIONS                            │
│ ─────────────    │ ──────────────────────────────────────── │
│ .TITAN .VERA     │ [Format All]  [Type Check]               │
│ .HELIX .AETHER   │ [Generate Docs] [Axiom Verify]           │
│ .AXIOM .SYLVA    │ [Convert Existing Code →]                │
│ .NEXUS           │                                          │
│                  │ SYMBOL SEARCH                            │
│ Filename:        │ ┌──────────────────────────────────────┐ │
│ [__________]     │ │ Search symbols...                    │ │
│ [Create File]    │ └──────────────────────────────────────┘ │
│                  │                                          │
│ PROJECT SCAFFOLD │ RECENT FILES                            │
│ ─────────────    │ ──────────────────────────────────────── │
│ [CLI App]        │ 📄 main.titan        2m ago             │
│ [Desktop App]    │ 📄 WebServer.aether  14m ago            │
│ [Web Server]     │ 📄 UserService.titan 1h ago             │
│ [ML Pipeline]    │ 📄 model.sylva       Yesterday          │
│ [Full Stack]     │                                          │
└──────────────────┴──────────────────────────────────────────┘
```

### Features

**File Scaffolding**: creates a real file in the workspace via `fs.writeFile` with the correct template for each language:

| Extension | Template Contents |
|---|---|
| `.titan` | `module {Name};\n\nfn main() -> i32 {\n    return 0;\n}\n` |
| `.vera` | `component {Name} {\n    props{},\n    state{\n        title: "{Name}"\n    },\n    render{\n        <View>\n            <Text>{state.title}</Text>\n        </View>\n    }\n}\n` |
| `.helix` | `shader vertex {Name}Vert {\n    inputs { position: vec4, uv: vec2 },\n    outputs { fragUv: vec2 },\n    code {\n        gl_Position = position;\n        fragUv = uv;\n    }\n}\n` |
| `.aether` | `actor {Name} {\n    message Start {}\n    message Stop {}\n\n    handler Start(msg) {\n        log("{Name} started");\n    }\n\n    handler Stop(msg) {\n        self.terminate();\n    }\n}\n` |
| `.axiom` | `theorem {Name}Safety {\n    preconditions {\n        // define preconditions\n    },\n    postconditions {\n        // define postconditions\n    },\n    invariants {\n        // define invariants\n    },\n    assertions {\n        // define assertions\n    }\n}\n` |
| `.sylva` | `model {Name} {\n    architecture: [\n        Dense { inputs: 128, outputs: 64, activation: relu },\n        Dense { inputs: 64,  outputs: 10,  activation: softmax }\n    ],\n    loss: cross_entropy,\n    optimizer: adam\n}\n` |
| `.nexus` | `layout {Name} {\n    breakpoints {\n        mobile:  480px,\n        tablet:  768px,\n        desktop: 1024px\n    },\n    flex { direction: column, gap: 16px }\n}\n` |

**Project Templates**: multi-file scaffolds created in one click:
- **CLI App**: `main.titan` + `src/lib.titan` + `BUILD.omnisystem`
- **Desktop App**: `main.titan` + `ui/App.vera` + `ui/styles.nexus` + `BUILD.omnisystem`
- **Web Server**: `actors/WebServer.aether` + `handlers/Routes.titan` + `BUILD.omnisystem`
- **ML Pipeline**: `model.sylva` + `data/Loader.titan` + `training/Trainer.aether` + `BUILD.omnisystem`
- **Formally Verified Module**: `lib.titan` + `proofs/Safety.axiom` + `BUILD.omnisystem`
- **Full Stack**: all of the above, wired together

**Quick Actions** (real operations):
- **Format All**: sends `build.start` with `--fmt-only` flag, shows diff of changes
- **Type Check**: sends `build.start` with `--check-only` flag, streams errors into a result panel
- **Generate Docs**: sends `build.start` with `--doc` flag, opens generated HTML in a preview pane
- **Axiom Verify**: sends `build.start` with `--verify-axiom` flag, streams theorem proof results

**Symbol Search**: keystroke-debounced calls to the LSP `workspace/symbol` request; results shown as a filterable list with file path and line number.

**Recent Files**: read from `~/.omnisystem/recent.json`, updated on every file open via `openFile` message. Clicking opens in VS Code.

### Persistence
`~/.omnisystem/state/studio.json`: `{selectedLang, lastTemplate, recentFiles[]}`

---

## App: OmniCC Build

### Purpose
The primary interface for compiling, running, and testing OmniOS projects. Shows real build output streamed live from the compiler.

### Layout
```
┌─────────────────────────────────────────────────────────────┐
│ ⚙️ OmniCC Build                              [● Building...] │
├──────────────┬──────────────────────────────────────────────┤
│ Target:      │ [▶ Build] [★ Release] [◈ WASM] [⚡ Run]      │
│ [x86_64-win] │ [✓ Test]  [👁 Watch]  [✕ Clean]              │
│ Opt:         │ ──────────────────────────────────────────── │
│ [O2 ▾]       │ PHASES    ████████░░░░░░░░░░░░░░░░  5/7     │
│              │ Parse ✓  Resolve ✓  Type ✓  Lower ✓  Opt ✓  │
│ ERRORS   0   │ Codegen ●  Link ○                           │
│ WARNINGS 2   │ ──────────────────────────────────────────── │
│              │ BUILD OUTPUT                                 │
│ ERROR LIST   │ ──────────────────────────────────────────── │
│ ─────────    │ [parse]   Parsing src/main.titan (142 LOC)  │
│ (none)       │ [parse]   Parsing src/utils.titan (87 LOC)  │
│              │ [resolve] 228 symbols resolved               │
│              │ [type]    Type checking passed               │
│              │ [lower]   Lowering to IR (384 instructions)  │
│              │ [opt]     Dead code elimination: -12 instr   │
│              │ [codegen] ▓▓▓▓▓▓▓▓░░ Generating...          │
└──────────────┴──────────────────────────────────────────────┘
│ BUILD HISTORY: ✓ O2 0.84s  ✓ O3 1.12s  ✗ O2 0.31s         │
```

### Features
- **Real-time output**: every line from `build.line` notifications appears instantly
- **Phase progress bar**: accurate — each `build.phase start/done` event updates the display
- **Error navigator**: left panel lists all errors and warnings grouped by file; clicking opens the file at the exact line in VS Code via `vscode.window.showTextDocument`
- **Clickable errors**: error lines in the output terminal are also clickable
- **Target and optimization selectors**: real values passed to the build command
- **Watch mode**: `build.watch` IPC — green spinner in taskbar chip during watch, red on error
- **Test runner**: `omnicc test --verbose` via `build.start` with test flags; per-test pass/fail/skip shown with timing
- **Build history**: last 10 builds persisted in `~/.omnisystem/builds.log`, shown in bottom strip with outcome icon, duration, and artifact size
- **Artifact list**: on successful build, shows output files with size and checksum; click to open file location in Files app
- **Cancel**: sends `build.cancel` IPC; runtime terminates the compiler subprocess cleanly

### IPC Commands Used
`build.start`, `build.cancel`, `build.watch`

### Persistence
`~/.omnisystem/state/build.json`: `{lastTarget, lastOpt, history[]}`

---

## App: Bonsai Hub

### Purpose
Manage and launch the entire Bonsai Ecosystem — a separate application suite that extends OmniOS to mobile, browser, and native desktop contexts.

### Layout
```
┌─────────────────────────────────────────────────────────────┐
│ 🌿 Bonsai Hub                                      v2.0     │
│ The Complete App Ecosystem                                   │
├─────────────────────────┬───────────────────────────────────┤
│ 🖥️ Launcher  ● Running  │ 📱 Buddy       ● Paired          │
│ ─────────────────────── │ ──────────────────────────────── │
│ Tauri desktop app with  │ Android companion — task sync,   │
│ system tray, auto-start │ remote build, file browser        │
│                         │                                   │
│ [Launch App] [Status]   │ [Connect] [Build APK]            │
│                         │                                   │
│ Launcher PID: 12345     │ Device: Pixel 7 Pro               │
│ Uptime: 14m 22s         │ Signal: ████████ Excellent        │
├─────────────────────────┼───────────────────────────────────┤
│ 🌐 Extension  ○ Stopped │ ⚡ Control Panel ● Online         │
│ ─────────────────────── │ ──────────────────────────────── │
│ Chrome + Firefox: web   │ Web UI at localhost:8080 —       │
│ capture, Bonsai search  │ unified dashboard, metrics, logs  │
│                         │                                   │
│ [Build] [Install Dev]   │ [Open Dashboard]                 │
└─────────────────────────┴───────────────────────────────────┘
│ [🌿 Open Bonsai Dashboard]                                  │
│ STATUS: Launcher ● | Buddy ● | Extension ○ | Panel ●        │
```

### Features

**Launcher Component:**
- Extension host finds the Bonsai executable (configurable path, auto-detected from `%APPDATA%\Bonsai\`, `/Applications/Bonsai.app`, `~/.local/share/Bonsai/`)
- Spawns the Bonsai Tauri app as a child process, monitors its PID
- Status dot reflects real process state: running (green), stopped (grey), crashed (red)
- "Status" button shows process uptime, memory usage, and crash logs

**Android Buddy:**
- Extension host runs `adb devices` to enumerate connected Android devices
- Shows real device list with connection quality
- "Connect" opens the ADB pairing flow
- "Build APK" runs the Bonsai Android build script, streams output in the Terminal app

**Browser Extension:**
- "Build Extension" runs the Bonsai extension build pipeline (webpack/esbuild), streams output
- Writes the output `.crx` / `.xpi` to a user-selected directory
- "Install Dev" provides step-by-step instructions with the actual output path filled in

**Control Panel:**
- Opens a VS Code webview pointing to `http://localhost:8080`
- If server is not running, "Start Server" button spawns `bonsai-control-panel --port 8080` via the extension host
- Real HTTP health check every 10 seconds to confirm the panel is responding

**Status row at bottom:**
- Every 10 seconds, the extension host runs health checks: process liveness for Launcher and Buddy, HTTP ping for Control Panel, build status for Extension
- Results update the status dots in real time

### IPC Commands Used
Custom commands: `bonsai.launchApp`, `bonsai.status`, `bonsai.kill`, `bonsai.buildApk`, `bonsai.buildExtension`, `bonsai.openPanel`

---

## App: ML Studio

### Purpose
Design, train, and export machine learning models using the Sylva language. Accessible to beginners (visual layer builder) and experts (direct Sylva source editing).

### Features
- **Visual model builder**: add/remove/reorder layers via drag-and-drop; generates a live `.sylva` model file as the architecture changes
- **Layer types**: Dense, Conv2D, MaxPool, Dropout, BatchNorm, LSTM, GRU, Attention, Embedding, Flatten
- **Hyperparameter editor**: learning rate, batch size, epochs, optimizer (adam/sgd/rmsprop/adamw), loss function (cross_entropy, mse, mae, huber)
- **Dataset browser**: file picker backed by `fs.readDir` — supports CSV, JSON Lines, Parquet (read via Sylva runtime)
- **Real training**: `ml.train` IPC sends the Sylva model and dataset to the runtime, which compiles the model to optimized WASM+SIMD and runs the training loop
- **Live metrics chart**: Canvas-based real-time chart of loss and accuracy per epoch from `ml.epoch` notification stream
- **Hyperparameter sweep**: define ranges (e.g., lr: [0.001, 0.01, 0.1]) — OmniOS schedules multiple training runs in parallel via Aether actors, shows a comparison table
- **Model export**: after training, exports to `.sylva-model` (native) or `.onnx` (cross-platform)
- **Pre-built model zoo**: one-click import of ResNet-18, BERT-base, GPT-2-small, YOLOv5-small as Sylva files with pre-trained weights
- **Stop training**: sends `ml.stop` IPC, cleanly terminates the training subprocess and saves checkpoint

### IPC Commands Used
`ml.train`, `ml.stop`, `fs.readDir` (for dataset browser), `fs.writeFile` (save model file)

---

## App: OmniPM

### Purpose
Manage packages for OmniOS projects — discover, install, update, audit, and remove packages from the OmniPM registry.

### Features
- **Installed tab**: all packages from `~/.omnisystem/packages/`, with name, version, description, size, install date; click to see full manifest
- **Registry tab**: search the live OmniPM registry — returns real results with download counts, versions, verified badge
- **Install**: progress bar backed by `pm.progress` notification stream showing download, verify, and extract phases
- **Dependency tree**: before installing, shows the full dependency graph — what will be installed and why
- **Lock file**: all installs update `omnisystem.lock` with pinned versions for reproducible builds
- **Audit**: `pm.audit` checks all installed packages against the OmniPM vulnerability database; shows CVE IDs and fix versions
- **Update all**: `pm.update` checks for newer versions of all installed packages, streams update progress
- **Private registry**: configurable registry URL in Settings for enterprise or private package hosting
- **Project view**: separate section showing only packages declared in the current project's `BUILD.omnisystem`

### IPC Commands Used
`pm.list`, `pm.search`, `pm.install`, `pm.remove`, `pm.update`, `pm.audit`

---

## App: App Converter

### Purpose
Convert existing software written in other languages to OmniOS Omni-Language equivalents. A 4-stage guided pipeline: Analyze → Plan → Execute → Verify.

### Stage 1 — Analyze
- User selects a source file or directory via a file picker (backed by `fs.readDir`)
- Selects source language: JavaScript/TypeScript, Python, C/C++, Rust, Java, C#, Go, Other
- `convert.analyze` IPC call runs the appropriate Tree-sitter parser
- Returns a semantic map: functions, classes, async patterns, UI components, imports
- Shows: item count by category, estimated difficulty, mapping plan with confidence scores

### Stage 2 — Plan
- User reviews the generated plan as a table: source construct → target construct → confidence
- Can override individual mappings (e.g., change a class from `struct` to a different pattern)
- "Notes" column shows warnings for items requiring manual review

### Stage 3 — Execute
- User selects output directory
- `convert.execute` IPC starts the transformation
- Progress stream shows each item as it is converted
- Items marked `manual` are written as `// TODO: manual review — [reason]` comments in the output
- All output files are written to the selected directory via `fs.writeFile`

### Stage 4 — Verify
- Automatically runs `build.start` with `--check-only` on the converted output
- Shows type errors as a list: each error links to the converted file at the specific line
- "Fix suggestions" for common conversion errors (e.g., "missing return type annotation")

### Language Transformation Rules

| Source | Target | Key Transformations |
|---|---|---|
| `class Foo {}` | `struct Foo {}` (Titan) | Methods become `impl Foo { fn method() }` |
| `async function f()` | `actor A { handler M(msg) {} }` (Aether) | Return values become response messages |
| `React.FC<Props>` | `component C { props{}, render{} }` (Vera) | JSX → Vera render tree |
| `useState(x)` | `state { x: T }` (Vera) | Setter calls → state mutations |
| `numpy.array()` | `tensor<f32>()` (Sylva) | NumPy ops → Sylva tensor ops |
| `trait Foo {}` | `protocol Foo {}` (Titan) | Near 1:1 with Rust |
| `interface Foo {}` | `protocol Foo {}` (Titan) | Near 1:1 with Java/C# |

---

## App: Settings

### Purpose
Configure all aspects of OmniOS and the VS Code extension. Every setting change takes real effect immediately.

### Sections

**Appearance:**
- Color theme selector (applies via `vscode.workspace.getConfiguration().update('workbench.colorTheme', ...)`)
- Desktop wallpaper style (aurora intensity, color scheme)
- Font family and size for desktop UI

**Language Server:**
- LSP enabled/disabled (actually starts/stops the language client — not just a toggle visual)
- Inlay hints (registers/deregisters the inlay hints provider)
- Axiom background verification (starts/stops the background theorem prover worker)
- Format on save (registers/deregisters the document formatting provider)
- Diagnostic delay (ms): how long after a keystroke before diagnostics are published (default 300ms)

**Build Configuration:**
- Default build target (written to `BUILD.omnisystem`)
- Default optimization level
- Parallel compilation workers (number of concurrent frontend threads)
- Build cache location

**OmniCC:**
- Path to `omnicc.cmd` (file picker with validation — runs `omnicc --version` on the selected file)
- Path to `omnicc runtime` executable
- Runtime IPC timeout (seconds)

**OmniPM:**
- Registry URL (default: https://registry.omnisystem.dev)
- Cache directory
- Auto-update check on startup

**Keyboard Shortcuts:**
- Shows current extension keybindings in a read-only list
- "Edit in VS Code" button opens the keybinding editor filtered to Omnisystem

### IPC Commands Used
`system.setConfig`, `system.getConfig`

---

## App: System Monitor

### Purpose
Real-time view of the OmniOS runtime's health, resource usage, and operational status.

### Features
- **Stats cards**: active systems (152/152), language count (7), total LOC (47K+), LSP status (live connection check)
- **CPU usage**: reads `os.cpus()` from the extension host, shows per-core usage as animated bar charts
- **Memory usage**: reads `process.memoryUsage()` — heap used, heap total, RSS, external
- **Runtime metrics** (from `system.stats` IPC): active Aether actors, message queue depth, GC pause times, event loop lag
- **Service health**: real ping to each service every 10 seconds; results update status dots in real time
- **Process list**: all `omnicc` subprocesses currently running (builds, tests, etc.) with their PID, CPU%, and command line
- **Build history**: read from `~/.omnisystem/builds.log` — last 10 builds with outcome, duration, target, artifact size
- **Diagnostics**: "Run Diagnostics" button sends `build.start` with `--check --system`, streams the self-diagnostic report
- **LSP latency**: shows real hover and completion latency from the language client's performance metrics
- **GC log**: last 20 garbage collections with pause duration

### IPC Commands Used
`system.stats`, `system.health`
