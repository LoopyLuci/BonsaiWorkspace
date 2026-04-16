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

