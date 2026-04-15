# Bonsai Workspace: Android App + VSCode Runner/Streaming System

## Overview

Two tightly related systems built on top of the existing Bonsai Workspace (Tauri 2 + SvelteKit):

1. **Bonsai Android App** — Tauri Mobile build that connects to the running desktop instance over LAN, providing chat, file browsing, and editor access from a phone.
2. **VSCode Runner/Streamer System** — A VSCode extension that streams semantic state (file tree, editor content, cursor, diagnostics) to Bonsai over WebSocket, rendered in Bonsai's own UI components. Bidirectional: Bonsai can send commands back to VSCode.

Both systems share the same WebSocket infrastructure added to the existing Axum server.

---

## Phase 1 — WebSocket Server (Axum, port 11371)

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
3. Desktop Settings panel shows: token in large text + QR code encoding `bonsai://connect?ip=192.168.x.x:11371&token=ABC123`.
4. Android scans QR → extracts IP + token → connects to `ws://IP:11371/ws` → sends `{"type":"auth","payload":{"token":"ABC123"}}`.
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
- On `activate()`: attempt WebSocket connect to `ws://127.0.0.1:11371/ws`.
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
// Manages ws://127.0.0.1:11371/ws connection
// On message: routes to appropriate store update
// Exports: sendVscodeCmd(cmd, args)
```

### App.svelte Integration
Add a "VSCode" tab in the left sidebar (or as a collapsible panel). `VscodeViewer` mounts when tab is active. The `wsClient` connects on app startup regardless.

---

## Phase 5 — Bonsai Android App (Tauri Mobile)

### Setup
1. Run `cargo tauri android init` to generate `src-tauri/gen/android/`.
2. Add Android-specific Cargo feature flags to skip desktop-only deps:

```toml
# Cargo.toml
[features]
desktop = ["scrap", "enigo", "portable-pty", "cpal"]
mobile = []
default = ["desktop"]

[target.'cfg(not(target_os = "android"))'.dependencies]
scrap = "0.5"
enigo = "0.0.14"
portable-pty = "0.8"
cpal = { version = "0.15", optional = true }
```

3. Gate desktop-only modules with `#[cfg(not(target_os = "android"))]`.

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
- Android app connects to desktop `ws://[desktop-ip]:11371/ws` using the token from QR scan.
- Tauri command `save_desktop_connection(ip: String, token: String)` persists to SQLite.
- On startup, loads saved connection and auto-reconnects.
- QR scan uses `@tauri-apps/plugin-barcode-scanner`.

### Android-Specific Commands
- `scan_qr() -> String` — invokes barcode scanner plugin, returns URL string.
- `save_desktop_connection(ip, token)` — persist to DB.
- `load_desktop_connection() -> Option<{ip, token}>` — load from DB.

### Tauri Plugins (Android)
Add to `Cargo.toml`:
```toml
tauri-plugin-barcode-scanner = "2"
tauri-plugin-os = "2"
```

---

## Phase 6 — mDNS Discovery (Optional Enhancement)

Instead of QR code only, allow Android to auto-discover desktop on same LAN.

- Add `mdns-sd = "0.11"` to `Cargo.toml`.
- Desktop registers `_bonsai._tcp.local.` service on port 11371 at startup.
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
