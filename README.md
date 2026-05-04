 # Bonsai Workspace

 Local-first AI development workspace built with Tauri 2, Svelte, Rust, and on-device model sidecars.

 This repository bundles a full desktop IDE, a native Rust backend for local automation and device tooling, and optional on-device model sidecars so you can iterate without cloud credentials.

 This README summarizes the product features, developer quick-starts, and where to find detailed docs in the repository.

 ## What's New

 - Model Data system with rich metadata and AI-assisted model profile generation for local and catalog models.
 - Quick Options dropdown in chat for Weather, Time, Files, Sys Stats, and Web actions.
 - Task queue with priority-aware scheduling and resource gating so chat/tool requests can be processed safely under load.
 - BonsaiBot multi-platform support across Discord, Telegram, Email, and Matrix.
 - BonsaiExeLauncherBuilder scripts for repeatable local build and packaging workflows.

 ## Quick Start

 1. Launch the desktop stack:

 ```powershell
 node bonsai-workspace/src/launch-all.mjs --mode desktop
 ```

 2. Open Model Selector and choose a model to load.
 3. Start chatting with Bonsai Buddy in the chat panel.

 ## Building from Source

 Use the launcher builder scripts from the repository root to produce desktop artifacts:

 ```powershell
 .\BonsaiExeLauncherBuilder.ps1
 ```

 The builder script runs frontend and Tauri build steps, then resolves and stages the built executable. See launcher options in the script help and `bonsai-workspace/launcher_manual.md`.

 ## Highlights

 - Multi-pane IDE with file tree, Monaco editor, integrated terminal, command palette, status bar, and activity-first logging.
 - Assistant & Bonsai Buddy: an integrated assistant system (chat, assistant profiles, TTS, saved sessions) with a detachable always-on-top Buddy window.
 - BonsaiBot: a lightweight messaging bot server for Discord/Telegram/Matrix/Email with an admin API used by the workspace.
 - Multi-agent swarm orchestration for orchestrating many small agents with leader/worker semantics, retries, and resource gating.
 - Mobile tooling: Android USB Lab, QR mobile pairing, Mobile Viewer (scrcpy integration), remote surface streaming & input.
 - Rich tooling: editor tool profiles (lint/format/test), per-language commands, Agent Vision, Agent Connect and plugin tooling.

 ## Quick Start (Windows)

 From the repository root (recommended):

 ```powershell
 cd Z:\Projects\BonsaiWorkspace\bonsai-workspace
 # Start the local Rust bot (admin API)
 .\bonsai-bot\target\release\bonsai-bot.exe

 # In another shell: run the desktop app (dev)
 npx tauri dev
 ```

 Or use the provided launchers from the workspace root:

 ```powershell
 .\Launch-BonsaiWorkspace.cmd            # one-click start
 .\Launch-BonsaiWorkspace.ps1           # PowerShell variant
 ```

 Common modes:

 ```powershell
 .\Launch-BonsaiWorkspace.cmd -Mode desktop+usb
 .\Launch-BonsaiWorkspace.cmd -Mode desktop+usb -RemoteSurfaceSmoke
 ```

 Developer flow (frontend + Tauri):

 ```bash
 cd bonsai-workspace/src
 npm install
 # from the workspace root Tauri finds src-tauri/tauri.conf.json
 cd ..
 npx tauri dev
 ```

 If you prefer building a production bundle:

 ```bash
 cd bonsai-workspace/src
 npm run build
 cd ../src-tauri
 cargo tauri build
 ```

 ## Key Components & Features

 ### Editor & Explorer

 - File tree with quick create, filter, and context actions.
 - Monaco editor with language autodetection, autosave, inline completions, and diff hunk apply/reject.
 - Per-language tooling profiles (format, lint, test, run) with persisted templates and placeholders.

 ### Assistant, Bonsai Buddy & Session Tools

 - Full featured assistant with:
   - persistent profiles and avatars,
   - saved chat sessions and session history,
   - approval-gated tool calls and replayable tool traces,
   - TTS playback and voice synthesis management.
 - `Bonsai Buddy` — detachable assistant window controlled via Tools → Bonsai Buddy or `toggle_assistant_window` Tauri command.

 ### BonsaiBot (Messaging Bot)

 - A small server (`bonsai-bot`) that provides an admin API for messaging integrations and automation.
 - Implements platform adapters (Discord, Telegram, Matrix, Email) and exposes tests and configuration via the app Settings.
 - Port discovery and token storage use OS keyring and a persisted port probe file (`bonsai-bot-port.json`).
 - See `bonsai-bot/MAIL_SERVER_PROD_PLAN.md` for the mail server rollout plan and integration notes.

 ### Multi-Agent Swarm, Agent Vision & Agent Connect

 - Persona and agent config CRUD for multi-agent workflows.
 - Leader/worker orchestration with runtime controls, token streaming, and debug event emission.
 - `Agent Vision` for image/video analysis workflows and `Agent Connect` for remote session orchestration.

 ### Mobile Tooling & Android USB Lab

 - `Android USB Lab` provides a guided readiness flow: detect device, check authorization, configure reverse port, install APKs, bootstrap connection, and run regression suites.
 - `Mobile Viewer` uses `scrcpy` (when available) to mirror and control a connected Android device; when scrcpy is missing the UI lists candidate executable paths resolved by the backend to aid troubleshooting.
 - Remote Surface: a web-accessible frame + input endpoints for device streaming and input routing.

- Mobile automation server: supports optional token-based authentication. The Testing Toolkit UI exposes a "Server auth token" field to supply the token when required; see `SECURITY.md` for recommended token storage and rotation practices.

Requirements for Android workflows: `adb` (Android platform tools) and, for screen mirroring, `scrcpy` installed on the host.

 ### Model Orchestration & Sidecars

 - Local model orchestration for `llama-server` style backends and optional whisper/tts sidecars.
 - Sidecar binaries live under `src-tauri/binaries` when present (platform-suffixed); models are stored in the platform data directory.
 - The app can operate in degraded mode without any sidecars for editing and orchestration tasks.

 ### Terminal, PTY & Activity Log

 - Multi-tab PTY terminal sessions with an Activity Log tab that streams app events, tool-call traces, and diagnostics.

- Activity Log instrumentation: all interactive UI controls are annotated with `data-bonsai-action` (format "Area:Action") so user interactions are captured with structured labels. Clickable elements in draggable titlebar regions are set to `-webkit-app-region: no-drag` to preserve interactivity.

 ### Settings, Secrets & Security

 - Settings panel supports API host/port configuration, bot platform secrets, and keyring-backed credential storage.
 - `assistant_commands` expose Tauri calls for SMTP secrets (`set_smtp_credentials`, `has_smtp_credentials`, `clear_smtp_credentials`).

 ## Running & Testing

 Run the backend bot for local integration tests:

 ```powershell
 cd bonsai-bot
 cargo run --release   # or run the built binary in target/release
 # admin API listens on 127.0.0.1:11424 by default
 ```

 Start the desktop app (recommended from the `bonsai-workspace` root so Tauri finds `src-tauri/tauri.conf.json`):

 ```powershell
 cd Z:\Projects\BonsaiWorkspace\bonsai-workspace
 npx tauri dev
 ```

 Mobile/USB smoke tests:

 ```bash
 # in separate shell
 cd bonsai-workspace/src
 npm run test:android-usb-regression
 ```

 ## Docs & Where to Look

 - User-facing guides: `bonsai-workspace/user_manual.md` and `bonsai-workspace/launcher_manual.md`.
 - Developer notes: `Runner-Streaming_System.md`, `Cluster-Orchestrator-Design.md`, `Multi-Agent_Swarm.md`.
 - Mail server plan: `bonsai-bot/MAIL_SERVER_PROD_PLAN.md`.

 ## Contributing

 Please open PRs against `main` and follow the repository's CI checks. See `.github/workflows` for CI details.

 ---

 If you'd like, I can also add a short quickstart README specifically for contributors (dev-only steps and checks). Would you like that next?

