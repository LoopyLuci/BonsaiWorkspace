# OmniOS Desktop — Complete Architecture

## Overview

OmniOS is a first-principles operating system platform built entirely on the Omni-Languages ecosystem (Titan, Vera, Helix, Aether, Axiom, Sylva, Nexus). It runs in six deployment modes — all from a single codebase. The VS Code extension is the primary power-user interface. Personal use and zero-knowledge users are equally first-class.

---

## System Layers

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  PRESENTATION LAYER                                                         │
│  VS Code Extension (Webview) │ Tauri Standalone App │ WASM Browser App     │
├─────────────────────────────────────────────────────────────────────────────┤
│  APPLICATION LAYER                                                          │
│  Files │ Terminal │ Code Studio │ Bonsai Hub │ OmniCC Build │ ML Studio    │
│  OmniPM │ App Converter │ Settings │ System Monitor                        │
├─────────────────────────────────────────────────────────────────────────────┤
│  SERVICE LAYER                                                              │
│  152 Omnisystem Services │ OmniPM Registry │ LSP Server │ Build Scheduler  │
│  Bonsai Ecosystem │ IPC Router │ State Persistence │ Notification Bus      │
├─────────────────────────────────────────────────────────────────────────────┤
│  RUNTIME LAYER                                                              │
│  OmniCC Compiler Pipeline │ VM Executor │ Garbage Collector                │
│  Aether Actor Scheduler │ Event Loop │ Module Linker                       │
├─────────────────────────────────────────────────────────────────────────────┤
│  FOUNDATION LAYER                                                           │
│  Omni-Languages: Titan · Vera · Helix · Aether · Axiom · Sylva · Nexus    │
│  Native Bindings: GPU (Helix) · Input (Titan) · Display (Vera)             │
└─────────────────────────────────────────────────────────────────────────────┘
```

Every component — from the window manager to the memory allocator — is written in an Omni-Language. TypeScript exists only as the VS Code extension host bridge: the thin boundary between VS Code's Electron/Node.js process and the OmniOS runtime. Everything inside that boundary is Omni-Languages.

---

## Deployment Modes

### 1. VS Code Extension (Primary Development Mode)
The extension host spawns `omnicc runtime --ipc` as a long-lived child process. The extension's webview panel provides the full OmniOS Desktop UI. All user actions in the webview travel through the extension host to the runtime via JSON-RPC 2.0 IPC. This is the primary mode for developers and power users.

### 2. Standalone Application (Tauri)
The Tauri shell embeds the OmniOS runtime and uses Vera's UI renderer directly (bypassing the webview layer). Ships as a native `.exe` / `.app` / `.deb`. Targets beginner and personal users who do not use VS Code.

### 3. Container (Docker / Podman)
OmniOS runtime runs headless. Exposes the same JSON-RPC 2.0 IPC protocol over a Unix socket or TCP port. A companion web UI connects to it. Used for CI/CD pipelines, cloud development environments, and DevOps workflows.

### 4. VM Image (QEMU)
A bootable disk image containing the GRUB bootloader, a minimal Linux kernel (or eventually the OmniOS native kernel), and the OmniOS runtime with all 152 systems. Used for sandboxed environments, education, and testing.

### 5. Bare Metal (Future Milestone)
UEFI bootloader written in Titan. OmniOS native kernel written in Titan + Helix (GPU) + Aether (I/O scheduling). Direct hardware access via the native bindings layer. This is a multi-year engineering effort begun after the other five modes are production-ready.

### 6. WASM Browser App (Zero-Install)
OmniCC compiled to WebAssembly via Binaryen. The full OmniOS Desktop runs in a browser tab at omnisystem.dev. No installation required. Uses OPFS (Origin Private File System) for persistence. Primary entry point for zero-knowledge users discovering OmniOS for the first time.

---

## Extension Host Architecture

```
VS Code Extension Host (Node.js / TypeScript)
│
├── extension.ts              Activation, command registration
│
├── runtime/
│   ├── RuntimeProcess.ts     Spawns and restarts omnicc runtime --ipc
│   ├── RuntimeClient.ts      JSON-RPC 2.0 client, message routing, request queue
│   └── RuntimeProtocol.ts    Typed interfaces for all IPC message schemas
│
├── providers/
│   ├── OmniTreeProvider.ts   VS Code sidebar: project explorer
│   ├── BonsaiTreeProvider.ts VS Code sidebar: Bonsai ecosystem
│   ├── BuildTreeProvider.ts  VS Code sidebar: build history
│   └── PackageTreeProvider.ts VS Code sidebar: installed packages
│
├── webviews/
│   ├── OmniOSDesktop.ts      Main desktop panel — message routing to RuntimeClient
│   ├── BonsaiDashboard.ts    Bonsai-specific panel
│   └── BuildDashboard.ts     Standalone build output panel
│
└── lsp/
    └── LanguageClient.ts     LSP connection to omnicc lsp --stdio
```

### RuntimeProcess.ts Responsibilities
- Spawn `omnicc runtime --ipc` with correct working directory and environment
- Monitor process health via heartbeat ping every 5 seconds
- Auto-restart on crash with exponential backoff (500ms → 1s → 2s → 4s, max 3 attempts)
- Emit `runtimeReady` / `runtimeCrashed` / `runtimeRestarted` events for UI status indicators
- Gracefully terminate on VS Code extension deactivation

### RuntimeClient.ts Responsibilities
- Maintain a pending-request map: `requestId → {resolve, reject, timeout}`
- Route incoming messages: responses go to pending map, events (buildLine, termOutput, etc.) go to registered listeners
- All requests have a 30-second timeout (configurable per command)
- Serialize outgoing JSON-RPC frames: `Content-Length: N\r\n\r\n{...json...}`
- Parse incoming frames using the same Content-Length framing as LSP

---

## Data Flow: User Action to Real Result

Example: User clicks "Build" in the OmniCC Build app.

```
1. Webview JS: post('runBuild', {args: ['build', '--target', 'x86_64-windows', '--opt', 'O2']})

2. OmniOSDesktop._handleMessage(msg)
   → runtimeClient.sendRequest('build.start', {target, opt, flags})

3. RuntimeClient.ts
   → serializes JSON-RPC 2.0 request: {jsonrpc: '2.0', id: 42, method: 'build.start', params: {...}}
   → writes Content-Length frame to omnicc stdin

4. omnicc runtime --ipc (OmniOS Runtime Process)
   → reads and parses the frame
   → invokes the build pipeline
   → for each compiler phase, writes: {jsonrpc: '2.0', method: 'build.line', params: {phase, text, level}}
   → on completion: {jsonrpc: '2.0', id: 42, result: {code: 0, artifacts: ['target/x86_64-windows/main.wasm']}}

5. RuntimeClient.ts
   → receives build.line notifications → forwards to OmniOSDesktop via onBuildLine listener
   → receives response → resolves the pending promise

6. OmniOSDesktop.ts
   → receives build.line events → panel.webview.postMessage({type: 'buildLine', text, phase})
   → receives build.done → panel.webview.postMessage({type: 'buildDone', code: 0, artifacts})

7. Webview JS
   → handleBuildLine(text) → appends to build output terminal in real time
   → handleBuildDone(code) → updates phase bar, badge, shows artifact list
```

Total latency from click to first output line: under 200ms.

---

## OmniOS Runtime Process (`omnicc runtime --ipc`)

This is the core server that backs the entire desktop. It runs as a persistent process for the lifetime of the VS Code session.

### Capabilities
- **File System**: real Node.js fs operations — readDir, readFile, writeFile (atomic via temp+rename), delete, move, watch (fs.watch streams change events)
- **Build System**: spawns the OmniCC compiler pipeline, streams phase events and output lines
- **Terminal**: spawns PTY instances via `node-pty`, bridges stdin/stdout/resize/kill
- **Package Manager**: HTTP calls to the OmniPM registry, local package extraction and verification
- **ML Training**: invokes the Sylva runtime (WASM or native), streams epoch metrics
- **App Conversion**: runs Tree-sitter parsers for source analysis, applies transformation rules
- **System Stats**: reads `process.memoryUsage()`, `os.cpus()`, spawns health-check subprocesses
- **State Persistence**: reads/writes `~/.omnisystem/state/*.json`

### Process Isolation
Each long-running operation (build, test, ML training, app conversion) runs as its own subprocess under the runtime's supervision. The runtime tracks all child process IDs and can cancel any operation cleanly. A crashed build subprocess does not crash the runtime or the desktop.

---

## Window Manager Architecture

### Z-Order
Windows are tracked in an ordered array `zStack: string[]` of app IDs. When a window is focused, its ID moves to the end of the array. Z-index values are assigned as array indices × 10, leaving gaps for sub-elements. This prevents the classic integer-overflow z-index bug from simple incrementing.

### State
Each window entry: `{id, title, icon, x, y, w, h, minimized, maximized, prevRect}`. The entire state is serialized to `~/.omnisystem/state/desktop.json` on every change and loaded on startup. The desktop reopens exactly as it was left.

### Snap Zones
While dragging, if the window center crosses within 20px of a screen edge, a ghost overlay appears showing the snap target. On mouseup inside the snap zone, the window snaps to fill that screen half or quarter. Snapping is reversible: the pre-snap rect is saved and restored if the user drags away.

### Virtual Desktops
Up to 9 virtual desktops. Each has its own `zStack` and window set. The taskbar shows the current desktop number. `Ctrl+Alt+Left/Right` switches desktops. Windows can be moved between desktops via the taskbar chip right-click jump list.

---

## Security Model

### Webview Content Security Policy
```
default-src 'none';
script-src 'nonce-{nonce}';
style-src 'nonce-{nonce}';
img-src data: vscode-resource:;
font-src data:;
```
No external network requests from the webview. All data comes through the extension host message channel. This prevents XSS and data exfiltration from compromised webview content.

### File System Access
The runtime process runs with the same OS-level permissions as the VS Code extension host (user permissions). It does not run as root. Sensitive operations (delete, overwrite) require an explicit user confirmation step — the webview sends a `confirm` command, the extension host shows a VS Code modal dialog, and only proceeds if the user clicks "OK".

### Package Verification
Every OmniPM package is verified against its published SHA-256 checksum before extraction. The registry uses HTTPS with certificate pinning. Packages run in the OmniCC VM, not as native processes, until the user explicitly marks them as trusted executables.
