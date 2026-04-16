# Bonsai Workspace

Local-first AI development workspace built with Tauri 2, Svelte, Rust, and on-device model sidecars.

No cloud model keys are required. Core inference, editing, orchestration, and tooling workflows can run fully local.

## What You Get

- Multi-pane IDE experience: file tree, Monaco editor, integrated terminal, chat, command palette, status bar.
- Agent-assisted coding with approval-gated tool execution.
- Multi-agent swarm mode with leader/worker orchestration and runtime controls.
- Activity-first terminal with shell tabs plus a dedicated diagnostic event log.
- Local API server with OpenAI-compatible endpoints for agent automation.
- Android USB Lab and mobile QR pairing flows for device-level validation and evidence capture.

## Quick Start

### Windows one-click launch from workspace root

From Z:/Projects/BonsaiWorkspace:

```powershell
.\Launch-BonsaiWorkspace.cmd
```

Common examples:

```powershell
.\Launch-BonsaiWorkspace.cmd -PreflightOnly
.\Launch-BonsaiWorkspace.cmd -Mode desktop+usb -StrictApp -ApkPath "C:\path\to\app.apk" -Serial "DEVICE_SERIAL"
```

### Recommended launcher from src

From bonsai-workspace/src:

```bash
npm run launch:preflight
npm run launch:desktop
npm run launch:desktop+usb
```

### Standard development flow

```bash
cd src
npm install
cd ../src-tauri
cargo tauri dev
```

## Prerequisites

| Tool | Minimum | Notes |
|---|---:|---|
| Rust | 1.77+ | Use rustup |
| Node.js | 20+ | Frontend + scripts |
| Tauri CLI | 2.x | cargo install tauri-cli --version "^2" |

Platform dependencies:

- Windows: Visual Studio Build Tools (Desktop development with C++) and WebView2 runtime.
- macOS: xcode-select --install.
- Ubuntu/Debian: webkit2gtk, gtk3, sqlite3, ssl, audio and build dependencies.

## Core Architecture

Project layout:

- src-tauri: Rust backend, command handlers, model orchestration, PTY management, API server, persistence.
- src: Svelte frontend, stores, Monaco integration, chat/terminal/settings UX.

Key systems:

- Command broker: Tauri command layer for editor, file system, git, terminal, remote, agent, and swarm operations.
- Tool-call loop: assistant emits tool calls, app executes with HITL approval where required, returns tool results into chat context.
- PTY sessions: per-tab shell sessions for integrated terminal tabs.
- Activity log: structured diagnostic stream from tool use, swarm events, permission requests, and runtime errors.
- Swarm orchestrator: leader plans subtasks, workers execute in parallel/sequential mode, leader synthesizes final output.

## Major Features

### Editor and Explorer

- File tree with open folder, filter, refresh, quick create file/folder, and context menu create actions.
- File-type detection and icon mapping for known extensions and special filenames.
- Monaco editor with:
  - language auto-detection from path,
  - auto-save,
  - diff hunk accept/reject overlays,
  - Ask Bonsai context actions,
  - inline completions.

### Tooling Profiles in Editor

- Per-language tooling profiles (web, python, rust, powershell, shell, config, docs, data, generic).
- One-click actions from editor header:
  - Load Tools
  - Lint
  - Format
  - Test
- Commands are profile-configurable and persisted locally.
- Placeholder support in command templates:
  - {file}
  - {dir}
  - {workspace}

### Chat and Agent Runtime

- Streaming chat with token-speed telemetry.
- Approval cards for sensitive operations.
- File-aware diff previews and patch apply controls.
- System-info hardening for RAM/spec requests:
  - deterministic tool-call enforcement,
  - fallback to approval request if model does not emit valid call.

### Multi-Agent Swarm

- Persona CRUD and agent slot configuration.
- Leader/worker orchestration with runtime controls:
  - planning requirement,
  - worker tool access,
  - parallel workers,
  - retries/timeouts,
  - synthesis style,
  - token streaming,
  - debug event emission.
- RAM safety gate before swarm execution.

### Terminal and Diagnostics

- Multi-tab shell terminal sessions.
- First tab is Activity Log with:
  - live event ingestion,
  - filter/search,
  - dedupe and max retention,
  - compact mode,
  - copy/save/clear.

### Settings, Remote, Mobile, USB

- API host/port settings with save/test/copy endpoint controls.
- Remote session lifecycle and preview hooks.
- Mobile QR pairing, saved connection verification, evidence records.
- Android USB Lab for readiness, install/launch, reverse/bridge, full regression, and ledger append helpers.

## AI Sidecar Setup (Optional in Dev)

Bonsai can run without sidecars in development. For local inference/voice features, add sidecar binaries.

Expected binaries in src-tauri/binaries are platform-suffixed (for example x86_64-pc-windows-msvc.exe variants).

Download sources:

- llama.cpp releases for llama-server
- whisper.cpp releases for server (renamed to whisper-server)

Model location examples:

- Windows: %APPDATA%/bonsai-workspace/models
- macOS: ~/Library/Application Support/bonsai-workspace/models

## API and Test Automation

From src:

```bash
npm run test:agent-api
npm run test:agent-ui-hitl
npm run test:agent-ui-live-orchestrated
npm run test:agent-orchestrated
npm run test:agent-routing-ci
npm run test:bonsai-live-testing-feature
npm run test:bonsai-live-testing-feature:headless
npm run test:android-usb-regression
```

Artifacts:

- Launcher report: tool_test/launcher/latest.json
- USB regression report: tool_test/android-usb-regression/latest.json

## Build for Distribution

```bash
cd src-tauri
cargo tauri build
```

Installers are produced under src-tauri/target/release/bundle.

## Documentation

- End-user guide: user_manual.md
- Workspace orchestration notes: Multi-Agent_Swarm.md
- Runner and evidence details: Runner-Streaming_System.md

## Troubleshooting Highlights

- If launch reports exit code 1 but output shows API healthy and running, verify whether the launcher process was interrupted versus app failure.
- If API settings save fails, verify host is non-empty and port is 1..65535.
- If tool output seems missing, switch to terminal shell tabs or Activity Log for routed output visibility.
- If swarm run is denied for memory, reduce enabled workers or select smaller models.

