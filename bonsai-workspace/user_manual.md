# Bonsai Workspace User Manual

This manual is the complete user guide for Bonsai Workspace. It explains every visible feature, how each workflow functions, and how to operate the app safely and efficiently.

## 1) Product Overview

Bonsai Workspace is a local-first AI coding workspace that combines:

- File explorer and editor (Monaco)
- Integrated terminal with multiple shell tabs
- AI chat with tool-use approvals
- Multi-agent swarm execution
- Local API server for automation
- Mobile pairing and Android USB operations

Core design principle:

- Keep code, prompts, tool execution, and inference local by default.

## 2) Interface Tour

Main interface areas:

- Top toolbar: open core panels and global actions.
- File tree panel: open folder, browse, search, create files/folders.
- Editor panel: code editing, diff review, inline assist, tooling commands.
- Chat panel: conversational coding assistant with approvals.
- Terminal panel: shell sessions and activity diagnostics.
- Status bar: runtime indicators, branch/status, telemetry.
- Modal panels: Settings, Sessions, Agent Connect, Agents.

## 3) Workspace and File Management

### 3.1 Open a workspace

How:

1. Click Open Folder in the file tree header.
2. Choose a local folder.

What happens:

- Workspace name and git branch are shown.
- Files are loaded and sorted in tree order.
- Branch indicator updates from git query when available.

### 3.2 File tree features

Available controls:

- Open Folder
- + File
- + Folder
- Refresh
- Filter files input

Additional behaviors:

- Right-click context menu in the tree for New File/New Folder.
- Directory expansion/collapse with keyboard and click.
- Search mode includes matching files and ancestor folders so paths remain navigable.

### 3.3 File-type detection and icons

Bonsai detects file type using filename and extension.

Examples:

- TypeScript, JavaScript, Svelte, Vue
- Python
- Rust
- PowerShell and shell scripts
- JSON, YAML, TOML, INI
- Markdown, SQL, HTML/CSS
- Special names like Dockerfile, Makefile, .gitignore, .env, package.json

Outcomes:

- Correct file icon in tree.
- Correct language label in editor pill.
- Correct tooling profile selection for lint/format/load/test commands.

## 4) Editor (Monaco) Functions

### 4.1 Open and edit files

How it works:

- Selecting a file in tree triggers open signal to editor.
- Editor reads file content through backend read command.
- Language mode is set automatically from file type detection.

### 4.2 Auto-save

- Editor uses debounced save after content changes.
- Dirty indicator dot appears until save succeeds.

### 4.3 Diff review widgets

When a diff is available:

- Inline hunk controls appear above affected lines.
- You can Accept or Reject each hunk.
- Accepted hunks are applied and file is reloaded.

### 4.4 Ask Bonsai on selected code

Context actions include:

- Explain selection
- Fix selection
- Refactor selection

Prompt construction includes:

- File context
- Language id
- Selected code block

### 4.5 Inline completions

- Editor requests inline completion from backend inference pipeline.
- Works from local context around cursor.

## 5) Editor Tooling Profiles

Tooling profiles are per-language-group command templates.

Built-in profiles:

- Web / Node
- Python
- Rust
- PowerShell
- Shell
- Config Files
- Docs / Markdown
- Data / SQL
- Generic

### 5.1 Header actions

Buttons in editor header:

- Load Tools: install/setup tooling dependencies
- Lint: run profile lint command
- Format: run profile format command
- Test: run profile test command
- Tools: open profile editor

All commands run through integrated terminal routing.

### 5.2 Profile editor

For the active file profile you can configure:

- Enabled toggle
- Load command
- Lint command
- Format command
- Test command
- Language tools list

Supported placeholders in templates:

- {file}
- {dir}
- {workspace}

Persistence:

- Saved locally in browser storage.
- Reset All Tooling Profiles restores defaults.

## 6) Chat Panel and Assistant Behavior

### 6.5 Quick Options

Quick Options is a shortcut dropdown in the chat area for common assistant actions.

Available options:

- Weather: asks Bonsai Buddy for current weather details.
- Time: requests local date/time from runtime tools.
- Files: launches a file-focused request pattern for search/read operations.
- Sys Stats: fetches live machine metrics and hardware summary.
- Web: opens a web-fetch workflow for URL/page summaries.

Usage:

1. Click Quick Options in the chat area.
2. Pick an option.
3. Review the injected prompt and send.

### 6.1 Sending prompts

- Enter sends.
- Shift+Enter creates newline.
- Assistant streams tokens while thinking.

### 6.2 Tool use and approvals

For sensitive operations the assistant raises an approval card.

Common tool categories:

- File operations
- Shell commands
- Read/list/search workspace

Approval paths:

- Approve executes tool action.
- Deny blocks action and informs model loop.

### 6.3 System facts and specs requests

For machine-info requests (RAM, CPU, GPU, OS):

- Assistant is forced to execute a real command path.
- Preferred command alias is specs.
- If model does not emit valid tool call, deterministic fallback triggers approval request.

Goal:

- Return factual local machine output, not guessed values.

### 6.4 Message rendering

- Markdown rendering for assistant output.
- Agent badges shown when messages are associated with swarm slots.
- Tool usage and stats can be attached to assistant messages.

## 7) Multi-Agent Swarm System

Swarm mode enables multiple agents per request with leader/worker orchestration.

### 7.1 Roles

- Leader (slot 0): plans, coordinates, synthesizes.
- Workers (slot 1+): execute subtasks concurrently/sequentially.

### 7.2 Personas and agent config

Manage in Agents panel:

- Create/edit/delete personas
- Assign persona to agent slot
- Set label, emoji, color, model, enable/disable

### 7.3 Resource safety

Before swarm run:

- RAM estimate is calculated
- Distinct model memory counted once where shared
- Per-agent overhead added
- Run is blocked if estimate exceeds safety threshold

### 7.4 Runtime settings

Available controls include:

- Require leader planning
- Allow worker tools
- Enable cross-review
- Parallel workers
- Include summaries
- Retry failed workers
- Stream worker tokens
- Emit debug events
- Include original prompt
- Allow leader as worker
- Max subtasks
- Worker timeout
- Max worker response chars
- Synthesis style

Settings are persisted locally.

### 7.5 Live swarm UI

- In-progress worker token rows appear in chat.
- Leader and worker messages are visually distinct.
- Swarm completion clears pending state.

## 8) Terminal Panel

### 8.1 Shell tabs

- Multiple PTY-backed shell sessions.
- Add and close tabs.
- Command history per tab.
- Arrow-up/down history recall.

### 8.2 Activity Log tab

First tab is Activity Log, used for diagnostics.

Tracks events from:

- Tool execution
- Permission requests
- Swarm plan/complete/error
- Agent connect events
- Terminal output patterns
- Unhandled runtime errors

- UI actions: All interactive UI controls are annotated with `data-bonsai-action` (format "Area:Action") so user interactions are captured with structured labels in the Activity Log. Clickable controls in draggable titlebar regions are set to `-webkit-app-region: no-drag` to ensure they remain interactive.

Controls:

- Search
- Settings (levels/categories, max entries, compact mode, dedupe, autoscroll)
- Copy
- Save JSON snapshot
- Clear

## 9) Command Palette

Open with Ctrl+K or Cmd+K.

Used for quick navigation and invoking command actions across the app.

## 10) Settings Panel

Settings covers model/runtime/network/remote/mobile/USB features.

### 10.1 Model and hardware section

- List available models
- Switch active model
- Download catalog models
- Import local GGUF
- View hardware info
- Download whisper model

### 10.1.1 Model Selector

The Model Selector is the main model control surface.

Core actions:

- Browse installed models and Bonsai Catalog entries.
- Load or switch the active model.
- Monitor load progress (percentage and elapsed time).

Rich metadata shown in the selector:

- Tier icon: indicates model class/size band.
- Strength chips: capability hints such as Code, Math, Writing, Reasoning.
- Context window badge: maximum context length for prompts.
- RAM badge: estimated memory footprint for local execution.

State indicators:

- Active badge: model is loaded and routing requests.
- Loading indicator: model is currently warming/loading.
- Download/Use action for catalog models depending on installation state.

### 10.2 API settings

Fields:

- API host
- API port

Actions:

- Copy endpoint
- Test API
- Save API settings

Validation:

- Host must be non-empty.
- Port must be between 1 and 65535.

### 10.3 Remote session controls

- Start remote session
- Stop remote session
- Copy stream URL
- Send remote input tests
- Live preview stream handling

### 10.4 Mobile pairing

- Display pairing token
- Generate QR
- Scan mobile QR
- Save desktop connection
- Verify saved pairing

Evidence support:

- Writes pairing verification records.
- Exposes evidence path and recent record snapshot.

### 10.5 Android USB Lab

Operator workflow:

1. Refresh devices.
2. Check readiness.
3. Resolve or set APK path.
4. Install and launch app.
5. Bootstrap reverse/bridge.
6. Run full validation.
7. Append ledger evidence.

Status model:

- DISCONNECTED
- UNAUTHORIZED
- ONLINE
- READY

Strict mode:

- Fails run if required app launch/install criteria are not met.

### 10.6 Testing Toolkit & Server Auth

- Testing Toolkit UI: the Testing Toolkit now includes a `Server auth token` field in its settings/run dialog. If the mobile automation server is configured to require token authentication, enter the token here to allow the UI to authenticate to the server.

- Mobile automation server: the server supports optional token-based authentication; this allows lightweight access control for CI or shared runners. For security guidance (storage, rotation, and recommended policies) see `SECURITY.md`.

## 11) Agent Connect Panel

Provides timeline/session style view for structured agent operations.

Key capabilities:

- Start and end sessions
- Observe lifecycle events
- Track summaries and status over time

## 12) Session Management

Session panel supports chat session handling and restoration behavior.

Persistent session state includes:

- Current session id
- Current session title

## 13) Notifications and Status

### 13.1 Toast notifications

- Success/info/error notifications with safe overlay placement above bottom UI.

### 13.2 Status bar

- Git branch indicator
- API/connection status
- Token speed and runtime indicators

- Queue indicator: pending and active task counts for assistant/task execution.

## 13.3 Task Queue

The task queue coordinates assistant and tool workloads under resource constraints.

Queue indicator behavior:

- Pending: tasks waiting for an execution slot.
- Active: tasks currently executing.

Priority behavior:

- Higher-priority tasks are scheduled first.
- Resource gating may defer large tasks until sufficient capacity is available.
- Rapid user submissions can temporarily increase pending count, then drain as workers complete.

## 14) Local API and External Automation

Bonsai exposes a local API for automated testing and agent integration.

Capabilities include:

- Health/version endpoints
- Model listing endpoints
- Remote session endpoints
- Input/frame/stream endpoints

Compatible scripts in src include orchestrated smoke/liveness/HITL flows.

## 15) Launchers and Artifacts

### 15.1 Root launcher scripts

- Launch-BonsaiWorkspace.cmd
- Launch-BonsaiWorkspace.ps1
- Generate-BonsaiDesktopShortcut.cmd
- Generate-BonsaiDesktopShortcut.ps1

### 15.2 Reporting artifacts

- tool_test/launcher/latest.json
- tool_test/android-usb-regression/latest.json

Reports are designed for traceability, including failure cases.

## 16) Keyboard Shortcuts

- Ctrl+K or Cmd+K: open command palette
- Enter in chat: send
- Shift+Enter in chat: newline

## 17) Typical End-to-End Workflows

### 17.1 AI-assisted coding workflow

1. Open workspace.
2. Open file in editor.
3. Ask assistant in chat.
4. Review approval requests.
5. Apply or reject diff hunks.
6. Run lint/format/test from editor tooling buttons.
7. Observe logs in Activity tab.

### 17.2 Swarm analysis workflow

1. Open Agents panel.
2. Configure personas and workers.
3. Tune runtime settings.
4. Submit prompt in chat.
5. Observe worker streams and final synthesis.
6. Use Activity Log for execution diagnostics.

### 17.3 Device validation workflow

1. Open Settings.
2. Run mobile pairing verification if needed.
3. Run Android USB Lab readiness and regression.
4. Save and append evidence artifacts.

## 18) Troubleshooting

### App launch seems to fail with exit code but UI/API started

- Check launcher report JSON.
- Verify whether process was interrupted rather than crashed.

### API settings save error

- Confirm host is not blank.
- Confirm port is numeric and valid range.

### Tool command output not obvious

- Check terminal shell tab and Activity Log tab.

### Swarm blocked for RAM

- Disable workers or choose lower-RAM model assignments.

### Remote preview issues

- Restart remote session.
- Re-check local API host/port settings.

## 19) Safety and Security Model

- Sensitive tool actions use explicit approval requests.
- Paths are validated for traversal guards in backend commands.
- Local-first default avoids external data transfer for core inference.

## 20) Power User Notes

- Use tooling profiles to standardize project lint/format commands quickly.
- Keep Activity Log open during swarm tuning or tool-call troubleshooting.
- Use launch preflight and report artifacts for CI-style diagnostics.

## 21) Where to Go Next

- Read README.md for setup and script command catalog.
- Use Multi-Agent_Swarm.md for orchestration design detail.
- Use Runner-Streaming_System.md for evidence/runner process detail.

## 22) Setup, Build, and Run — Full Developer Guide

This section summarizes how to prepare a development machine, run tests, and start the Workspace, the local `bonsai-bot` service, and the Mobile Viewer (scrcpy) components.

### 22.1 System prerequisites

- Install Rust (stable toolchain) and Cargo: https://rustup.rs/
- Install Node.js (recommended 18.x or later) and npm or yarn.
- Install Tauri prerequisites (platform-specific). On Windows: Visual Studio Build Tools and Git.
- Install Android Platform Tools (ADB): https://developer.android.com/studio/releases/platform-tools
- (Optional) Install `scrcpy` for Mobile Viewer: https://github.com/Genymobile/scrcpy

On Windows, prefer adding `adb.exe` and `scrcpy.exe` to PATH; Bonsai will also probe common install locations and display candidate paths in the Mobile Viewer settings.

### 22.2 Build and run (developer loop)

1. Install frontend deps (once):

```bash
cd bonsai-workspace/src
npm ci
```

2. Start the frontend dev server (Vite):

```bash
npm --prefix "bonsai-workspace/src" run dev
```

Frontend dev server default URL: `http://localhost:1420` (check console output).

3. Start the native app / Tauri dev (optional - launches the desktop shell):

```bash
cd bonsai-workspace
npx --yes --package @tauri-apps/cli tauri dev
```

4. Start `bonsai-bot` (platform connectors, admin API):

```bash
cd bonsai-bot
cargo run --release
```

The `bonsai-bot` admin API listens by default on port `11421` (near-by fallbacks possible) and writes the resolved port to `bonsai-bot-port.json` in the standard config dir for local discovery.

5. Run the Tauri backend / workspace API (if not running via `tauri dev`) — see the repository README for the exact command. The workspace API default port is `11369`.

### 22.3 Running tests

From the Tauri crate (recommended):

```bash
cd bonsai-workspace/src-tauri
cargo test
```

From the bot service:

```bash
cd bonsai-bot
cargo test
```

Unit tests and quick integration checks are included for: policy rules, tool scheduler, memory store, context builder, skill executor, and mobile tooling helpers.

## 23) Bots: setup, platforms, and communication

## 23.0 BonsaiBot Setup

BonsaiBot provides messaging integration and admin control endpoints.

Supported platforms:

- Discord
- Telegram
- Matrix
- Email

Where to configure:

- Main config: OS config directory `bonsai-bot-config.json` (or working directory override).
- Secrets/tokens: platform sections in config and keyring-backed admin token handling.

Token placement guidance:

- Discord token under the Discord platform config block.
- Telegram bot token under Telegram config.
- Matrix credentials and homeserver URL under Matrix config.
- SMTP/IMAP credentials for Email in the Email config block.

Validation:

1. Start `bonsai-bot`.
2. Check `/health` for liveness.
3. Check authenticated `/status` for per-platform connection state.

`bonsai-bot` is the local platform gateway and adapter that connects the Workspace to chat platforms (Discord, Telegram, Matrix, Email) and to the Buddy local API.

### 23.1 Configuration

- `bonsai-bot` reads `bonsai-bot-config.json` in the OS config dir (or the working directory). See `bonsai-bot/src/config.rs` for fields and defaults.
- Key fields to configure:
	- `admin_port` — admin API port (default 11421)
	- `buddy_api_url` — URL to the local Buddy API (default `http://127.0.0.1:11420`)
	- Per-platform slots: `discord`, `telegram`, `matrix`, `email` with credentials and allowed lists.

Example quick-start flow to enable a platform:

1. Edit `bonsai-bot-config.json` (or use the built-in admin endpoints) and enable e.g. `discord.enabled = true` and fill the platform `config` values.
2. Start `bonsai-bot`.
3. Use the admin API `/status` endpoint to confirm platform adapters are connected (requires admin token). The admin health endpoint `/health` is unauthenticated for quick checks.

### 23.2 Admin API & tokens

- On first run `bonsai-bot` ensures `bot_admin_token` in the OS keyring (see `src/config.rs`).
- The admin API exposes management endpoints under `/` such as `/status`, `/sessions`, `/broadcast` and `/config/rotate-admin-token`.
- `/status` requires `Authorization: Bearer <bot_admin_token>`; `/health` is open and can be used to detect if the admin API is listening.

To inspect health quickly:

```bash
curl -sS http://127.0.0.1:11421/health
# expected: { "status": "ok", "version": "..." }
```

If `/health` returns OK but the UI cannot reach bots, confirm `bonsai-bot` wrote `bonsai-bot-port.json` in your config dir and that the UI / workspace is configured to use the same admin endpoint.

### 23.3 Platform onboarding notes

- Discord: provide bot token and restrict `allowed_guild_ids` to the guild(s) you want to allow.
- Telegram: configure `allowed_chat_ids` and token; long-polling is used by default.
- Matrix: configure `homeserver_url` and user credentials.
- Email: configure IMAP/SMTP credentials and allowed-from addresses for inbound processing.

Each platform adapter writes connection status into the Admin API `/status` `platforms` map so the UI can render platform states in the Bots/settings panels.

## 24) Mobile Viewer (scrcpy) — finishing integration & usage

The Mobile Viewer lets you mirror and control Android devices via `scrcpy` and a small remote runtime on-device (reverse ports) to expose frames and input endpoints for the app.

### 24.1 Install and verify prerequisites

- Install `adb` (Android Platform Tools) and ensure it's on `PATH`.
- Install `scrcpy` for screen mirroring (on Windows, put `scrcpy.exe` in PATH or install through Chocolatey / Scoop). Bonsai probes common locations and shows candidate paths in the Mobile Viewer UI.

Verify ADB and scrcpy from your shell:

```bash
adb version
scrcpy --version
```

If `scrcpy` is not detected, the Mobile Viewer UI shows candidate locations that were probed by the backend and a link to the `scrcpy` docs.

### 24.2 Typical Mobile Viewer workflow

1. Connect device (USB or TCP/IP adb) and enable USB debugging.
2. Open `Mobile Viewer` from Tools → Mobile Viewer.
3. Click `Refresh` to populate devices.
4. Select device and click `Start Viewer` — this launches `scrcpy` locally and starts the remote surface runtime on-device (reverse ports).
5. Optionally click `Prepare Runtime` to ensure ADB reverse ports are set, Bonsai app is launched on device, and remote surface (frame/input) endpoints are exposed.
6. Use the on-screen controls to tap, swipe, type, take screenshots, or start/stop recordings.

Under the hood:

- The backend command `android_mobile_view_start` launches `scrcpy` with the selected flags. If the process cannot start the backend returns a helpful error listing candidate `scrcpy` paths checked.
- The backend command `android_mobile_prepare_uniform_runtime` performs wake/unlock, `adb reverse` for the API and WS ports, launches the Bonsai app, and optionally starts the Remote Surface activity that posts frames to the desktop API.

### 24.3 Troubleshooting Mobile Viewer

- If `Start Viewer` fails with `Failed to start scrcpy`, verify `scrcpy` is installed and accessible from the user context used to run Bonsai.
- If frames are missing but `scrcpy` runs, try `Prepare Runtime` to ensure reverse ports and Remote Surface activity are launched.
- If reverse port errors occur, ensure `adb reverse --list` shows the mapping and that the device supports reverse (some vendor OEM builds or older Android versions may not).

## 25) Quick verification steps (end-to-end)

1. Start frontend dev server (`npm --prefix bonsai-workspace/src run dev`) and confirm `http://localhost:1420` responds in a browser.
2. Start `bonsai-bot` (`cargo run` in `bonsai-bot`) and check `http://127.0.0.1:11421/health` returns OK.
3. Start the workspace backend (Tauri) or run `tauri dev` and open the Tools → Bots/Settings → Bots tab.
4. The Bots tab queries the admin endpoints and the workspace API; if both services are running the Bots tab will show connected platforms, statuses, and allow platform-specific configuration.

## 26) Files and locations

- Frontend code: `bonsai-workspace/src`
- Tauri backend: `bonsai-workspace/src-tauri/src`
- Bots service: `bonsai-bot/src`
- Config examples: `bonsai-bot/bonsai-bot-config.json` (created in the config dir on first run)

## 27) Contact and contribution notes

If you extend platform adapters or add new skill types, add unit tests under the corresponding crate and update `user_manual.md` with the new platform connection steps and security considerations.


# Bonsai Workspace User Manual

This manual is the complete user guide for Bonsai Workspace. It explains every visible feature, how each workflow functions, and how to operate the app safely and efficiently.

## 1) Product Overview

Bonsai Workspace is a local-first AI coding workspace that combines:

- File explorer and editor (Monaco)
- Integrated terminal with multiple shell tabs
- AI chat with tool-use approvals
- Multi-agent swarm execution
- Local API server for automation
- Mobile pairing and Android USB operations

Core design principle:

- Keep code, prompts, tool execution, and inference local by default.

## 2) Interface Tour

Main interface areas:

- Top toolbar: open core panels and global actions.
- File tree panel: open folder, browse, search, create files/folders.
- Editor panel: code editing, diff review, inline assist, tooling commands.
- Chat panel: conversational coding assistant with approvals.
- Terminal panel: shell sessions and activity diagnostics.
- Status bar: runtime indicators, branch/status, telemetry.
- Modal panels: Settings, Sessions, Agent Connect, Agents.

## 3) Workspace and File Management

### 3.1 Open a workspace

How:

1. Click Open Folder in the file tree header.
2. Choose a local folder.

What happens:

- Workspace name and git branch are shown.
- Files are loaded and sorted in tree order.
- Branch indicator updates from git query when available.

### 3.2 File tree features

Available controls:

- Open Folder
- + File
- + Folder
- Refresh
- Filter files input

Additional behaviors:

- Right-click context menu in the tree for New File/New Folder.
- Directory expansion/collapse with keyboard and click.
- Search mode includes matching files and ancestor folders so paths remain navigable.

### 3.3 File-type detection and icons

Bonsai detects file type using filename and extension.

Examples:

- TypeScript, JavaScript, Svelte, Vue
- Python
- Rust
- PowerShell and shell scripts
- JSON, YAML, TOML, INI
- Markdown, SQL, HTML/CSS
- Special names like Dockerfile, Makefile, .gitignore, .env, package.json

Outcomes:

- Correct file icon in tree.
- Correct language label in editor pill.
- Correct tooling profile selection for lint/format/load/test commands.

## 4) Editor (Monaco) Functions

### 4.1 Open and edit files

How it works:

- Selecting a file in tree triggers open signal to editor.
- Editor reads file content through backend read command.
- Language mode is set automatically from file type detection.

### 4.2 Auto-save

- Editor uses debounced save after content changes.
- Dirty indicator dot appears until save succeeds.

### 4.3 Diff review widgets

When a diff is available:

- Inline hunk controls appear above affected lines.
- You can Accept or Reject each hunk.
- Accepted hunks are applied and file is reloaded.

### 4.4 Ask Bonsai on selected code

Context actions include:

- Explain selection
- Fix selection
- Refactor selection

Prompt construction includes:

- File context
- Language id
- Selected code block

### 4.5 Inline completions

- Editor requests inline completion from backend inference pipeline.
- Works from local context around cursor.

## 5) Editor Tooling Profiles

Tooling profiles are per-language-group command templates.

Built-in profiles:

- Web / Node
- Python
- Rust
- PowerShell
- Shell
- Config Files
- Docs / Markdown
- Data / SQL
- Generic

### 5.1 Header actions

Buttons in editor header:

- Load Tools: install/setup tooling dependencies
- Lint: run profile lint command
- Format: run profile format command
- Test: run profile test command
- Tools: open profile editor

All commands run through integrated terminal routing.

### 5.2 Profile editor

For the active file profile you can configure:

- Enabled toggle
- Load command
- Lint command
- Format command
- Test command
- Language tools list

Supported placeholders in templates:

- {file}
- {dir}
- {workspace}

Persistence:

- Saved locally in browser storage.
- Reset All Tooling Profiles restores defaults.

## 6) Chat Panel and Assistant Behavior

### 6.1 Sending prompts

- Enter sends.
- Shift+Enter creates newline.
- Assistant streams tokens while thinking.

### 6.2 Tool use and approvals

For sensitive operations the assistant raises an approval card.

Common tool categories:

- File operations
- Shell commands
- Read/list/search workspace

Approval paths:

- Approve executes tool action.
- Deny blocks action and informs model loop.

### 6.3 System facts and specs requests

For machine-info requests (RAM, CPU, GPU, OS):

- Assistant is forced to execute a real command path.
- Preferred command alias is specs.
- If model does not emit valid tool call, deterministic fallback triggers approval request.

Goal:

- Return factual local machine output, not guessed values.

### 6.4 Message rendering

- Markdown rendering for assistant output.
- Agent badges shown when messages are associated with swarm slots.
- Tool usage and stats can be attached to assistant messages.

## 7) Multi-Agent Swarm System

Swarm mode enables multiple agents per request with leader/worker orchestration.

### 7.1 Roles

- Leader (slot 0): plans, coordinates, synthesizes.
- Workers (slot 1+): execute subtasks concurrently/sequentially.

### 7.2 Personas and agent config

Manage in Agents panel:

- Create/edit/delete personas
- Assign persona to agent slot
- Set label, emoji, color, model, enable/disable

### 7.3 Resource safety

Before swarm run:

- RAM estimate is calculated
- Distinct model memory counted once where shared
- Per-agent overhead added
- Run is blocked if estimate exceeds safety threshold

### 7.4 Runtime settings

Available controls include:

- Require leader planning
- Allow worker tools
- Enable cross-review
- Parallel workers
- Include summaries
- Retry failed workers
- Stream worker tokens
- Emit debug events
- Include original prompt
- Allow leader as worker
- Max subtasks
- Worker timeout
- Max worker response chars
- Synthesis style

Settings are persisted locally.

### 7.5 Live swarm UI

- In-progress worker token rows appear in chat.
- Leader and worker messages are visually distinct.
- Swarm completion clears pending state.

## 8) Terminal Panel

### 8.1 Shell tabs

- Multiple PTY-backed shell sessions.
- Add and close tabs.
- Command history per tab.
- Arrow-up/down history recall.

### 8.2 Activity Log tab

First tab is Activity Log, used for diagnostics.

Tracks events from:

- Tool execution
- Permission requests
- Swarm plan/complete/error
- Agent connect events
- Terminal output patterns
- Unhandled runtime errors

Controls:

- Search
- Settings (levels/categories, max entries, compact mode, dedupe, autoscroll)
- Copy
- Save JSON snapshot
- Clear

## 9) Command Palette

Open with Ctrl+K or Cmd+K.

Used for quick navigation and invoking command actions across the app.

## 10) Settings Panel

Settings covers model/runtime/network/remote/mobile/USB features.

### 10.1 Model and hardware section

- List available models
- Switch active model
- Download catalog models
- Import local GGUF
- View hardware info
- Download whisper model

### 10.2 API settings

Fields:

- API host
- API port

Actions:

- Copy endpoint
- Test API
- Save API settings

Validation:

- Host must be non-empty.
- Port must be between 1 and 65535.

### 10.3 Remote session controls

- Start remote session
- Stop remote session
- Copy stream URL
- Send remote input tests
- Live preview stream handling

### 10.4 Mobile pairing

- Display pairing token
- Generate QR
- Scan mobile QR
- Save desktop connection
- Verify saved pairing

Evidence support:

- Writes pairing verification records.
- Exposes evidence path and recent record snapshot.

### 10.5 Android USB Lab

Operator workflow:

1. Refresh devices.
2. Check readiness.
3. Resolve or set APK path.
4. Install and launch app.
5. Bootstrap reverse/bridge.
6. Run full validation.
7. Append ledger evidence.

Status model:

- DISCONNECTED
- UNAUTHORIZED
- ONLINE
- READY

Strict mode:

- Fails run if required app launch/install criteria are not met.

## 11) Agent Connect Panel

Provides timeline/session style view for structured agent operations.

Key capabilities:

- Start and end sessions
- Observe lifecycle events
- Track summaries and status over time

## 12) Session Management

Session panel supports chat session handling and restoration behavior.

Persistent session state includes:

- Current session id
- Current session title

## 13) Notifications and Status

### 13.1 Toast notifications

- Success/info/error notifications with safe overlay placement above bottom UI.

### 13.2 Status bar

- Git branch indicator
- API/connection status
- Token speed and runtime indicators

## 14) Local API and External Automation

Bonsai exposes a local API for automated testing and agent integration.

Capabilities include:

- Health/version endpoints
- Model listing endpoints
- Remote session endpoints
- Input/frame/stream endpoints

Compatible scripts in src include orchestrated smoke/liveness/HITL flows.

## 15) Launchers and Artifacts

### 15.1 Root launcher scripts

- Launch-BonsaiWorkspace.cmd
- Launch-BonsaiWorkspace.ps1
- Generate-BonsaiDesktopShortcut.cmd
- Generate-BonsaiDesktopShortcut.ps1

### 15.2 Reporting artifacts

- tool_test/launcher/latest.json
- tool_test/android-usb-regression/latest.json

Reports are designed for traceability, including failure cases.

## 16) Keyboard Shortcuts

- Ctrl+K or Cmd+K: open command palette
- Enter in chat: send
- Shift+Enter in chat: newline

## 17) Typical End-to-End Workflows

### 17.1 AI-assisted coding workflow

1. Open workspace.
2. Open file in editor.
3. Ask assistant in chat.
4. Review approval requests.
5. Apply or reject diff hunks.
6. Run lint/format/test from editor tooling buttons.
7. Observe logs in Activity tab.

### 17.2 Swarm analysis workflow

1. Open Agents panel.
2. Configure personas and workers.
3. Tune runtime settings.
4. Submit prompt in chat.
5. Observe worker streams and final synthesis.
6. Use Activity Log for execution diagnostics.

### 17.3 Device validation workflow

1. Open Settings.
2. Run mobile pairing verification if needed.
3. Run Android USB Lab readiness and regression.
4. Save and append evidence artifacts.

## 18) Troubleshooting

### App launch seems to fail with exit code but UI/API started

- Check launcher report JSON.
- Verify whether process was interrupted rather than crashed.

### API settings save error

- Confirm host is not blank.
- Confirm port is numeric and valid range.

### Tool command output not obvious

- Check terminal shell tab and Activity Log tab.

### Swarm blocked for RAM

- Disable workers or choose lower-RAM model assignments.

### Remote preview issues

- Restart remote session.
- Re-check local API host/port settings.

## 19) Safety and Security Model

- Sensitive tool actions use explicit approval requests.
- Paths are validated for traversal guards in backend commands.
- Local-first default avoids external data transfer for core inference.

## 20) Power User Notes

- Use tooling profiles to standardize project lint/format commands quickly.
- Keep Activity Log open during swarm tuning or tool-call troubleshooting.
- Use launch preflight and report artifacts for CI-style diagnostics.

## 21) Where to Go Next

- Read README.md for setup and script command catalog.
- Use Multi-Agent_Swarm.md for orchestration design detail.
- Use Runner-Streaming_System.md for evidence/runner process detail.

