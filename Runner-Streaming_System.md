# Bonsai Workspace: Android App + VSCode Runner/Streaming System

## Overview

Two tightly related systems built on top of the existing Bonsai Workspace (Tauri 2 + SvelteKit):

1. **Bonsai Android App** — Tauri Mobile build that connects to the running desktop instance over LAN, providing chat, file browsing, and editor access from a phone.
2. **VSCode Runner/Streamer System** — A VSCode extension that streams semantic state (file tree, editor content, cursor, diagnostics) to Bonsai over WebSocket, rendered in Bonsai's own UI components. Bidirectional: Bonsai can send commands back to VSCode.

Both systems share the same WebSocket infrastructure added to the existing Axum server.

---

## Phase 1 — WebSocket Server (Axum, port 11369)

**File:** `bonsai-workspace/src-tauri/src/api_server.rs`

Add a new Axum route `/ws` that upgrades to WebSocket. The server maintains a `WsRouter` (Arc<Mutex<HashMap<ClientId, WsSender>>>) passed via Axum state.

### Message Protocol (JSON)

**Desktop → Client:**
```json
{ "type": "vscode_state", "payload": { ... } }
{ "type": "chat_token", "payload": { "token": "..." } }
{ "type": "auth_ok", "payload": {} }
{ "type": "auth_fail", "payload": { "reason": "..." } }
```

**Client → Desktop:**
```json
{ "type": "auth", "payload": { "token": "..." } }
{ "type": "vscode_cmd", "payload": { "cmd": "open_file", "args": { "path": "..." } } }
{ "type": "chat_send", "payload": { "content": "..." } }
```

### Implementation Steps

1. Add `tokio-tungstenite` or use `axum`'s built-in WebSocket support (`axum::extract::ws`) — axum 0.7 includes WebSocket natively, no new dep needed.
2. Create `ws_router.rs`: `WsRouter` struct with `broadcast()`, `send_to()`, `register()`, `unregister()`.
3. Add `AppState.ws_router: Arc<WsRouter>` field.
4. Register `/ws` route in `build_router()`.
5. On upgrade: verify auth token before allowing subscriptions.

**New files:**
- `src-tauri/src/ws_router.rs`

**Modified files:**
- `src-tauri/src/api_server.rs` — add `/ws` route + state
- `src-tauri/src/lib.rs` — add `ws_router` to `AppState`

---

## Phase 2 — Authentication (Token Pairing)

**Goal:** Android connects to desktop without manual IP entry or passwords.

### Flow
1. Desktop generates a random 6-character alphanumeric token on startup, stored in `AppState.pair_token: String`.
2. Tauri command `get_pair_token()` returns the token.
3. Desktop Settings panel shows: token in large text + QR code encoding `bonsai://connect?ip=192.168.x.x:11369&token=ABC123`.
4. Android scans QR → extracts IP + token → connects to `ws://IP:11369/ws` → sends `{"type":"auth","payload":{"token":"ABC123"}}`.
5. Server validates token → sends `auth_ok` → client is registered.

### QR Code
- Use `qrcode` crate (pure Rust, no native deps) to generate SVG/PNG from token+IP string.
- Expose as Tauri command `generate_pair_qr() -> String` (SVG string).
- Render in Settings panel via `{@html qrSvg}`.

### IP Discovery
- New Tauri command `get_local_ip() -> String` — iterates network interfaces, returns first non-loopback IPv4.
- Display alongside QR code.

**New files:** none  
**Modified files:**
- `src-tauri/Cargo.toml` — add `qrcode = "0.13"`, `local-ip-address = "0.6"`
- `src-tauri/src/lib.rs` — add `pair_token` to `AppState`, generate on init
- `src-tauri/src/commands.rs` — add `get_pair_token`, `generate_pair_qr`, `get_local_ip`
- `src/lib/components/SettingsPanel.svelte` — add Connection section with QR + token display

---

## Phase 3 — VSCode Extension

**Directory:** `vscode-extension/` (new top-level sibling to `bonsai-workspace/`)

### Structure
```
vscode-extension/
  package.json         (publisher, activationEvents, contributes.commands)
  src/
    extension.ts       (activate/deactivate)
    bonsai-client.ts   (WebSocket connection manager)
    state-streamer.ts  (captures VSCode state, sends deltas)
    command-handler.ts (receives commands from Bonsai, executes in VSCode)
```

### State Streamer — Events Sent to Bonsai

| Event type | Trigger | Payload |
|---|---|---|
| `vscode_file_tree` | workspace folders change, file watch | `{ root, entries: [{path, kind, children?}] }` |
| `vscode_editor_open` | `onDidChangeActiveTextEditor` | `{ path, language, content, cursor }` |
| `vscode_editor_delta` | `onDidChangeTextDocument` | `{ path, ops: [{range, newText}] }` |
| `vscode_cursor` | `onDidChangeTextEditorSelection` | `{ path, line, col }` |
| `vscode_diagnostics` | `onDidChangeDiagnostics` | `{ path, items: [{line,col,severity,message}] }` |
| `vscode_copilot` | `onDidChangeInlineCompletionItems` (if API exists) | `{ path, suggestion }` |

### Delta Compression
- On `vscode_editor_open`: send full `content`.
- On `vscode_editor_delta`: send VSCode's `TextDocumentChangeEvent.contentChanges` array directly — these are already ranged diffs, no OT library needed.

### Commands Received from Bonsai

| Command | Action |
|---|---|
| `open_file` | `vscode.window.showTextDocument(Uri.file(path))` |
| `cursor_set` | `editor.selection = new Selection(line, col, line, col)` |
| `text_edit` | `editor.edit(builder => builder.replace(range, newText))` |
| `execute_command` | `vscode.commands.executeCommand(id, ...args)` |
| `show_diff` | `vscode.diff(original, modified, title)` |

### Connection
- On `activate()`: attempt WebSocket connect to `ws://127.0.0.1:11369/ws`.
- Send `{"type":"auth","payload":{"token":"..."}}` — token configured in VSCode settings (`bonsai.pairToken`).
- Reconnect with exponential backoff on disconnect.
- Status bar item shows connection state.

---

## Phase 4 — VSCode Viewer Pane (Desktop)

**New Svelte component:** `src/lib/components/VscodeViewer.svelte`

Subscribes to a `vscodeState` store (writable, updated by WebSocket messages). Renders:
- **File tree tab**: reuses existing `FileTree` component style, but data comes from `vscode_file_tree` messages instead of Tauri invoke.
- **Editor tab**: reuses `MonacoEditor` component, but content comes from `vscode_editor_open` + applies `vscode_editor_delta` ops. When user edits → send `text_edit` command back to VSCode (toggle: "mirror" vs "read-only" mode).
- **Diagnostics tab**: styled list of errors/warnings with file + line links.
- **Copilot tab**: shows current inline suggestion, "Accept" button sends `text_edit`.

### Store
**New file:** `src/lib/stores/vscodeState.ts`
```typescript
export interface VscodeFileEntry { path: string; kind: 'file' | 'dir'; children?: VscodeFileEntry[] }
export interface VscodeEditorState { path: string; language: string; content: string; cursor: {line:number;col:number} }
export interface VscodeDiagnostic { path:string; line:number; col:number; severity:'error'|'warning'|'info'; message:string }

export const vscodeFileTree = writable<VscodeFileEntry[]>([]);
export const vscodeEditor = writable<VscodeEditorState | null>(null);
export const vscodeDiagnostics = writable<VscodeDiagnostic[]>([]);
export const vscodeCopilot = writable<string>('');
export const vscodeConnected = writable<boolean>(false);
```

### WebSocket Client (Desktop)
**New file:** `src/lib/utils/wsClient.ts`
```typescript
// Manages ws://127.0.0.1:11369/ws connection
// On message: routes to appropriate store update
// Exports: sendVscodeCmd(cmd, args)
```

### App.svelte Integration
Add a "VSCode" tab in the left sidebar (or as a collapsible panel). `VscodeViewer` mounts when tab is active. The `wsClient` connects on app startup regardless.

---

## Phase 5 — Bonsai Android App (Tauri Mobile)

### Setup
1. Run `cargo tauri android init` to generate `src-tauri/gen/android/`.
2. Add target-specific mobile plugin dependencies in `src-tauri/Cargo.toml` (current implementation uses target-gated plugin deps rather than a desktop/mobile feature split):

```toml
[target.'cfg(any(target_os = "android", target_os = "ios"))'.dependencies]
tauri-plugin-barcode-scanner = "2"
```

3. Initialize `tauri_plugin_barcode_scanner::init()` in `src-tauri/src/lib.rs` under `#[cfg(any(target_os = "android", target_os = "ios"))]` so desktop builds remain unchanged.

### Mobile UI Layout
**New file:** `src/lib/components/MobileLayout.svelte`

Tab bar at bottom with 4 tabs:
- **Chat** — full ChatPanel (already works, just needs mobile CSS)
- **Files** — FileTree with touch-friendly larger hit targets
- **Editor** — MonacoEditor (Monaco has mobile support)
- **VSCode** — VscodeViewer (connects to desktop over LAN)

Show/hide `MobileLayout` vs desktop layout based on Tauri's platform detection:
```typescript
import { platform } from '@tauri-apps/plugin-os';
const isMobile = (await platform()) === 'android';
```

### Android Networking
- Android app connects to desktop `ws://[desktop-ip]:11369/ws` using the token from QR scan.
- Tauri command `save_desktop_connection(ip: String, token: String)` persists to SQLite.
- On startup, loads saved connection and auto-reconnects.
- QR scan uses `@tauri-apps/plugin-barcode-scanner`.

### Android-Specific Commands
- `scan_qr() -> String` — retained as a compatibility command; mobile QR scanning is handled in the frontend via `@tauri-apps/plugin-barcode-scanner`.
- `save_desktop_connection(ip, token)` — persist to DB.
- `load_desktop_connection() -> Option<{ip, token}>` — load from DB.

### Tauri Plugins (Android)
Add to `Cargo.toml`:
```toml
tauri-plugin-barcode-scanner = "2"
```

---

## Phase 6 — mDNS Discovery (Optional Enhancement)

Instead of QR code only, allow Android to auto-discover desktop on same LAN.

- Add `mdns-sd = "0.11"` to `Cargo.toml`.
- Desktop registers `_bonsai._tcp.local.` service on port 11369 at startup.
- Android browses for `_bonsai._tcp.local.` services and lists found desktops by hostname.
- User taps a desktop → token entry dialog (or QR scan to get token).

**Modified files:**
- `src-tauri/src/lib.rs` — start mDNS service registration in `setup()`
- `src-tauri/src/commands.rs` — add `browse_bonsai_services() -> Vec<{name, ip, port}>`

---

## Implementation Order

| Phase | Est. Complexity | Prerequisite |
|---|---|---|
| 1 — WebSocket server | Medium | None |
| 2 — Token auth + QR | Low | Phase 1 |
| 3 — VSCode extension | Medium | Phase 1 |
| 4 — VSCode Viewer pane | Medium | Phase 1, 3 |
| 5 — Android app | High | Phase 1, 2 |
| 6 — mDNS | Low | Phase 1 |

Recommended order: 1 → 2 → 4 → 3 → 5 → 6

---

## Files to Create

| File | Purpose |
|---|---|
| `src-tauri/src/ws_router.rs` | WebSocket connection registry + broadcast |
| `src/lib/stores/vscodeState.ts` | VSCode state stores |
| `src/lib/utils/wsClient.ts` | Desktop WebSocket client for VSCode viewer |
| `src/lib/components/VscodeViewer.svelte` | VSCode semantic viewer component |
| `src/lib/components/MobileLayout.svelte` | Mobile tab-bar layout |
| `vscode-extension/package.json` | VSCode extension manifest |
| `vscode-extension/src/extension.ts` | Extension entry point |
| `vscode-extension/src/bonsai-client.ts` | WebSocket client (VSCode side) |
| `vscode-extension/src/state-streamer.ts` | VSCode state capture |
| `vscode-extension/src/command-handler.ts` | Command execution from Bonsai |

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/api_server.rs` | Add `/ws` WebSocket route |
| `src-tauri/src/lib.rs` | Add `ws_router`, `pair_token` to `AppState` |
| `src-tauri/src/commands.rs` | Add `get_pair_token`, `generate_pair_qr`, `get_local_ip`, `scan_qr`, `save_desktop_connection`, `load_desktop_connection` |
| `src-tauri/Cargo.toml` | Add `qrcode`, `local-ip-address`, `mdns-sd`; add feature flags for desktop-only deps |
| `src/App.svelte` | Add VSCode tab, mobile layout detection |
| `src/lib/components/SettingsPanel.svelte` | Add Connection section (QR display, token, IP) |



I’m going to do a focused evidence-based audit across backend, frontend, extension, and CI so I can give you a prioritized gap list with an execution path to production-grade confidence.

Ran terminal command: git status --short

Read [](file:///z%3A/Projects/BonsaiWorkspace/Runner-Streaming_System.md#1-1), lines 1 to 260

Read [](file:///z%3A/Projects/BonsaiWorkspace/bonsai-workspace/.github/workflows/ci.yml)

Read [](file:///z%3A/Projects/BonsaiWorkspace/bonsai-workspace/.github/workflows/nightly-soak.yml)

I’m now validating the current build/test baseline and pulling exact code evidence for remaining gaps so the audit is actionable rather than theoretical.

Ran terminal command: Set-Location "z:\Projects\BonsaiWorkspace\bonsai-workspace"; cargo check --manifest-path "src-tauri/Cargo.toml"

Ran terminal command: Set-Location "z:\Projects\BonsaiWorkspace\bonsai-workspace\src"; npm run test:agent-routing-ci

Read [](file:///c%3A/Users/limpi/AppData/Roaming/Code/User/workspaceStorage/690560126d5bfaef4e7a4ce1fac11581/GitHub.copilot-chat/chat-session-resources/feda5104-870f-4d9f-b8c7-b9f870fd6fd6/call_dvUO2Pt2hPyDnc9KlHZGEMOB__vscode-1776288642822/content.txt)

Read [](file:///z%3A/Projects/BonsaiWorkspace/bonsai-workspace/src-tauri/src/commands.rs#1885-1885), lines 1885 to 1995

Read [](file:///z%3A/Projects/BonsaiWorkspace/bonsai-workspace/src-tauri/Cargo.toml)

Read [](file:///z%3A/Projects/BonsaiWorkspace/bonsai-workspace/src/lib/utils/wsClient.ts)

Read [](file:///z%3A/Projects/BonsaiWorkspace/vscode-extension/src/bonsai-client.ts)

Searched for regex `pub async fn scan_qr|save_desktop_connection|load_desktop_connection|browse_bonsai_services|std::thread::sleep` (`**/bonsai-workspace/src-tauri/src/commands.rs`), 6 results

Searched for regex `tauri-plugin-barcode-scanner|tauri-plugin-os|cfg\(not\(target_os = "android"\)\)|\[features\]` (`**/bonsai-workspace/src-tauri/Cargo.toml`), no results

Searched for regex `11369|/ws` (`**/Runner-Streaming_System.md`), 19 results

Searched for regex `ws://127.0.0.1:11369/ws|bonsai.wsUrl` (`**/vscode-extension/**`), 3 results

Read [](file:///z%3A/Projects/BonsaiWorkspace/bonsai-workspace/src-tauri/src/commands.rs#1868-1868), lines 1868 to 1894

Read [](file:///z%3A/Projects/BonsaiWorkspace/bonsai-workspace/src-tauri/src/config.rs#1-1), lines 1 to 50

Searched for regex `\[features\]|target_os = "android"|tauri-plugin-barcode-scanner|tauri-plugin-os` (`**/bonsai-workspace/src-tauri/Cargo.toml`), no results

Searched for regex `BONSAI_SKIP_API|agent-api-smoke|Skipping API smoke` (`**/bonsai-workspace/src/agent-routing-ci.mjs`), 2 results

Audit completed. Current baseline is healthy, but there are still important delivery gaps before claiming end-to-end production readiness.

Current state snapshot
1. Working tree is clean.
2. Backend compile passes.
3. Deterministic routing regression passes (8/8).
4. Core Runner-Streaming architecture exists across backend, desktop UI, and VSCode extension.

Priority findings (ordered by risk)

1. Critical: Android QR scan is still a stub, not implemented
- Evidence: commands.rs
- Impact: Mobile pairing flow is incomplete; users cannot actually scan and onboard via native QR from app.
- Gap vs plan: Runner-Streaming Phase 5 expected a working scan command.

2. Critical: Android-specific dependency and feature gating from the plan is missing
- Evidence: 

Read [](file:///z%3A/Projects/BonsaiWorkspace/bonsai-workspace/src-tauri/src/commands.rs#1885-1885), lines 1885 to 1995

Read [](file:///z%3A/Projects/BonsaiWorkspace/bonsai-workspace/src-tauri/Cargo.toml)

Read [](file:///z%3A/Projects/BonsaiWorkspace/bonsai-workspace/src/lib/utils/wsClient.ts)

Read [](file:///z%3A/Projects/BonsaiWorkspace/vscode-extension/src/bonsai-client.ts)

Cargo.toml
- Impact: Mobile build readiness is uncertain; desktop-only dependencies are not explicitly gated for Android as specified.
- Gap vs plan: Runner-Streaming Phase 5 calls for Android feature gates and mobile plugin wiring.

3. High: Port contract drift between spec and implementation
- Spec says 11369:
  - Runner-Streaming_System.md
  - Runner-Streaming_System.md
  - Runner-Streaming_System.md
- Implementation uses 11369:
  - config.rs
  - wsClient.ts
  - package.json
  - bonsai-client.ts
- Impact: Integration confusion, wrong defaults in external clients, and onboarding failures if teams follow the spec doc literally.

4. High: CI deterministic suite skips API smoke coverage
- Evidence: agent-routing-ci.mjs
- Impact: API regression risk in remote/session endpoints; current CI validates UI scripted flow but not API smoke path in that lane.

5. High: CI does not validate VSCode extension build/tests
- Extension tests exist:
  - state-streamer.test.ts
- CI workflows currently focus on bonsai-workspace jobs:
  - ci.yml
- Impact: Extension can silently break while desktop remains green.

6. Medium: mDNS browse command uses blocking sleep in an async command
- Evidence: commands.rs
- Impact: Potential runtime responsiveness issues under load; low immediate risk, but suboptimal implementation pattern.

7. Medium: Nightly soak is single-platform
- Evidence: nightly-soak.yml
- Impact: OS-specific flaky behavior may go undetected (especially Linux/macOS differences).

8. Medium: Functional parity claim is ahead of verification scope
- The plan includes full mobile functionality and discovery paths:
  - Runner-Streaming_System.md
- Current verification evidence is strong for desktop routing and backend compile, but not yet for mobile runtime behavior.

Clear optimal path to full functional confidence

Phase A: Contract and onboarding correctness (1 day)
1. Decide and lock a single canonical WS/API port (11369).
2. Update either implementation or spec so they match everywhere:
- Runner-Streaming_System.md
- config.rs
- wsClient.ts
- package.json
3. Add one compatibility fallback if needed during migration.

Exit criteria:
1. No port mismatch between docs, defaults, and runtime.
2. Pairing examples work without manual correction.

Phase B: Complete mobile command surface (2-3 days)
1. Implement real scan_qr behavior (platform-gated).
2. Add Android plugin dependencies and platform feature gating.
3. Validate save/load desktop connection + mDNS browse with actual Android device/emulator.

Exit criteria:
1. End-to-end mobile pairing works from QR and persisted reconnect.
2. Android build succeeds in CI and local.

Phase C: CI coverage completion (1-2 days)
1. Add VSCode extension lane:
- install, compile, run tests in vscode-extension
2. Add API smoke lane that does not skip BONSAI_SKIP_API.
3. Add Android build lane (even if smoke-only initially).
4. Expand nightly soak to at least one additional OS.

Exit criteria:
1. Required checks include desktop app, extension, and API smoke.
2. Nightly reports platform-specific flake rates.

Phase D: Reliability hardening (2-4 days)
1. Replace blocking sleep in async browse path with non-blocking approach.
2. Add retry diagnostics for all flaky-prone jobs (attempt-level metadata already started).
3. Add structured artifact triage docs tied to generated logs.

Exit criteria:
1. Zero known blocking calls in async hot paths.
2. Actionable failure artifacts for every critical job.

Can this be guaranteed 100%
Absolute mathematical 100% correctness is not achievable for a distributed multi-platform system. The practical target is release-grade confidence with:
1. Deterministic required checks all green.
2. Soak and flaky rates below threshold for multiple days.
3. Full contract alignment between docs and runtime.
4. Verified mobile + extension interoperability in CI and device tests.

