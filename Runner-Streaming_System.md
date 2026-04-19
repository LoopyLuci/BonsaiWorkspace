# Bonsai Workspace: Android App + VSCode Runner/Streaming System

## Overview

Two tightly related systems built on top of the existing Bonsai Workspace (Tauri 2 + SvelteKit):

1. **Bonsai Android App** â€” Tauri Mobile build that connects to the running desktop instance over LAN, providing chat, file browsing, and editor access from a phone.
2. **VSCode Runner/Streamer System** â€” A VSCode extension that streams semantic state (file tree, editor content, cursor, diagnostics) to Bonsai over WebSocket, rendered in Bonsai's own UI components. Bidirectional: Bonsai can send commands back to VSCode.

Both systems share the same WebSocket infrastructure added to the existing Axum server.

---

## Progress Snapshot (April 2026)

Current status is strong for desktop and extension flows, with mobile runtime validation remaining as the primary gap.

### Latest validation signal

- Live visible watch suite passes end-to-end: 5/5 scenarios.
- Command used:

```powershell
npm --prefix "z:\Projects\BonsaiWorkspace\bonsai-workspace\src" run test:bonsai-live-testing-feature:watch
```

### Phase status

| Phase | Status | Notes |
|---|---|---|
| 1 â€” WebSocket server | Done | Stable and validated through repeated live runs. |
| 2 â€” Token auth + QR | Done (Desktop), Partial (Mobile runtime) | Desktop pairing/token/QR flows pass in live suite; real-device mobile scan validation still pending. |
| 3 â€” VSCode extension | Done | Extension lane and integration behavior are in place; desktop rendering + command loop works in current architecture. |
| 4 â€” VSCode Viewer pane | Done | Wired and functioning in the current UI architecture. |
| 5 â€” Android app | Partial | Mobile path implemented, but still needs full on-device validation pass and sign-off checklist. |
| 6 â€” mDNS | Partial | Implemented path exists; needs final runtime/behavior confidence pass under mobile conditions. |

### Remaining work to claim release-grade completion

1. Execute and document a real Android device validation pass for pairing, reconnect, and session continuity.
2. Run final mobile-focused soak pass for discovery and reconnect stability.
3. Mark each phase with objective exit criteria evidence (logs/artifacts) in this document after device validation.

---

## Phase 1 â€” WebSocket Server (Axum, port 11369)

**File:** `bonsai-workspace/src-tauri/src/api_server.rs`

Add a new Axum route `/ws` that upgrades to WebSocket. The server maintains a `WsRouter` (Arc<Mutex<HashMap<ClientId, WsSender>>>) passed via Axum state.

### Message Protocol (JSON)

**Desktop â†’ Client:**
```json
{ "type": "vscode_state", "payload": { ... } }
{ "type": "chat_token", "payload": { "token": "..." } }
{ "type": "auth_ok", "payload": {} }
{ "type": "auth_fail", "payload": { "reason": "..." } }
```

**Client â†’ Desktop:**
```json
{ "type": "auth", "payload": { "token": "..." } }
{ "type": "vscode_cmd", "payload": { "cmd": "open_file", "args": { "path": "..." } } }
{ "type": "chat_send", "payload": { "content": "..." } }
```

### Implementation Steps

1. Add `tokio-tungstenite` or use `axum`'s built-in WebSocket support (`axum::extract::ws`) â€” axum 0.7 includes WebSocket natively, no new dep needed.
2. Create `ws_router.rs`: `WsRouter` struct with `broadcast()`, `send_to()`, `register()`, `unregister()`.
3. Add `AppState.ws_router: Arc<WsRouter>` field.
4. Register `/ws` route in `build_router()`.
5. On upgrade: verify auth token before allowing subscriptions.

**New files:**
- `src-tauri/src/ws_router.rs`

**Modified files:**
- `src-tauri/src/api_server.rs` â€” add `/ws` route + state
- `src-tauri/src/lib.rs` â€” add `ws_router` to `AppState`

---

## Phase 2 â€” Authentication (Token Pairing)

**Goal:** Android connects to desktop without manual IP entry or passwords.

### Flow
1. Desktop generates a random 6-character alphanumeric token on startup, stored in `AppState.pair_token: String`.
2. Tauri command `get_pair_token()` returns the token.
3. Desktop Settings panel shows: token in large text + QR code encoding `bonsai://connect?ip=192.168.x.x:11369&token=ABC123`.
4. Android scans QR â†’ extracts IP + token â†’ connects to `ws://IP:11369/ws` â†’ sends `{"type":"auth","payload":{"token":"ABC123"}}`.
5. Server validates token â†’ sends `auth_ok` â†’ client is registered.

### QR Code
- Use `qrcode` crate (pure Rust, no native deps) to generate SVG/PNG from token+IP string.
- Expose as Tauri command `generate_pair_qr() -> String` (SVG string).
- Render in Settings panel via `{@html qrSvg}`.

### IP Discovery
- New Tauri command `get_local_ip() -> String` â€” iterates network interfaces, returns first non-loopback IPv4.
- Display alongside QR code.

**New files:** none  
**Modified files:**
- `src-tauri/Cargo.toml` â€” add `qrcode = "0.13"`, `local-ip-address = "0.6"`
- `src-tauri/src/lib.rs` â€” add `pair_token` to `AppState`, generate on init
- `src-tauri/src/commands.rs` â€” add `get_pair_token`, `generate_pair_qr`, `get_local_ip`
- `src/lib/components/SettingsPanel.svelte` â€” add Connection section with QR + token display

---

## Phase 3 â€” VSCode Extension

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

### State Streamer â€” Events Sent to Bonsai

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
- On `vscode_editor_delta`: send VSCode's `TextDocumentChangeEvent.contentChanges` array directly â€” these are already ranged diffs, no OT library needed.

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
- Send `{"type":"auth","payload":{"token":"..."}}` â€” token configured in VSCode settings (`bonsai.pairToken`).
- Reconnect with exponential backoff on disconnect.
- Status bar item shows connection state.

---

## Phase 4 â€” VSCode Viewer Pane (Desktop)

**New Svelte component:** `src/lib/components/VscodeViewer.svelte`

Subscribes to a `vscodeState` store (writable, updated by WebSocket messages). Renders:
- **File tree tab**: reuses existing `FileTree` component style, but data comes from `vscode_file_tree` messages instead of Tauri invoke.
- **Editor tab**: reuses `MonacoEditor` component, but content comes from `vscode_editor_open` + applies `vscode_editor_delta` ops. When user edits â†’ send `text_edit` command back to VSCode (toggle: "mirror" vs "read-only" mode).
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

## Phase 5 â€” Bonsai Android App (Tauri Mobile)

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
- **Chat** â€” full ChatPanel (already works, just needs mobile CSS)
- **Files** â€” FileTree with touch-friendly larger hit targets
- **Editor** â€” MonacoEditor (Monaco has mobile support)
- **VSCode** â€” VscodeViewer (connects to desktop over LAN)

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
- `scan_qr() -> String` â€” retained as a compatibility command; mobile QR scanning is handled in the frontend via `@tauri-apps/plugin-barcode-scanner`.
- `save_desktop_connection(ip, token)` â€” persist to DB.
- `load_desktop_connection() -> Option<{ip, token}>` â€” load from DB.

### Tauri Plugins (Android)
Add to `Cargo.toml`:
```toml
tauri-plugin-barcode-scanner = "2"
```

---

## Phase 6 â€” mDNS Discovery (Optional Enhancement)

Instead of QR code only, allow Android to auto-discover desktop on same LAN.

- Add `mdns-sd = "0.11"` to `Cargo.toml`.
- Desktop registers `_bonsai._tcp.local.` service on port 11369 at startup.
- Android browses for `_bonsai._tcp.local.` services and lists found desktops by hostname.
- User taps a desktop â†’ token entry dialog (or QR scan to get token).

**Modified files:**
- `src-tauri/src/lib.rs` â€” start mDNS service registration in `setup()`
- `src-tauri/src/commands.rs` â€” add `browse_bonsai_services() -> Vec<{name, ip, port}>`

---

## Implementation Order

| Phase | Est. Complexity | Prerequisite |
|---|---|---|
| 1 â€” WebSocket server | Medium | None |
| 2 â€” Token auth + QR | Low | Phase 1 |
| 3 â€” VSCode extension | Medium | Phase 1 |
| 4 â€” VSCode Viewer pane | Medium | Phase 1, 3 |
| 5 â€” Android app | High | Phase 1, 2 |
| 6 â€” mDNS | Low | Phase 1 |

Recommended order: 1 â†’ 2 â†’ 4 â†’ 3 â†’ 5 â†’ 6

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

---

## Release Readiness Checklist

Use this checklist to close remaining gaps and record objective evidence.

### Completed baseline

- [x] Desktop live visible validation passes (5/5 scenarios).
- [x] Repeatable watch-mode command validated.

```powershell
npm --prefix "z:\Projects\BonsaiWorkspace\bonsai-workspace\src" run test:bonsai-live-testing-feature:watch
```

### Remaining required checks

- [ ] Android QR pairing verified on a real device.
- [ ] Persisted reconnect verified after app restart (`save_desktop_connection` / `load_desktop_connection`).
- [ ] mDNS browse flow verified under mobile runtime conditions.
- [x] Final contract consistency pass (ports, routes, token docs vs runtime defaults). All ports confirmed at 11369.
- [x] USB-first Android desktop control implemented (install/launch/reverse/tcpip/connect/shell).
- [x] USB Lab Runtime System implemented (readiness state machine, APK resolver, install/launch orchestrator, connection bootstrap, full validation with schema_version 2 artifacts).
- [ ] Strict mode pass (`BONSAI_REQUIRE_APP=1` + valid APK) on physical device — blocked pending APK build.
- [ ] Manual CI hardware run with strict mode enabled — blocked pending APK build and self-hosted runner access.

### Real Device Verification Procedure

1. Launch desktop Bonsai and open Settings > Mobile & VSCode Connection.
2. On Android build, tap `Scan Mobile QR` and scan the desktop QR.
3. Confirm `Saved desktop connection` appears.
4. Confirm pairing verification result appears (`Pairing verified ... auth_ok`).
5. Tap `Verify Saved Pairing` to validate persisted reconnect path.
6. Restart mobile app and repeat step 5.

Evidence capture behavior:

- Each verification appends one JSON record to `mobile-pairing-evidence.jsonl` in app data.
- Settings displays `Evidence log` path and `Last evidence` snapshot.
- Record fields include timestamp, source (`qr_scan` or `saved_connection`), IP, verification result, websocket URL, elapsed milliseconds, and token hint.

### USB Lab Runtime System — Operator Runbook

Bonsai Desktop provides a complete USB Lab Runtime System in Settings → Android USB Lab.
A new operator can go from USB plug-in to PASS without terminal usage.

**One-click flow:**

1. Plug Android tablet in over USB and enable USB debugging on device.
2. Open Bonsai Desktop → Settings → Android USB Lab.
3. Click **Refresh Devices** — device appears in the dropdown.
4. Select the device serial and click **Check Readiness**.
   - Badge shows `DISCONNECTED` / `UNAUTHORIZED` / `ONLINE` / `READY`.
   - If `UNAUTHORIZED`: tap "Allow USB debugging" on device, then refresh.
5. Enter APK path or click **Resolve** to auto-discover from build output.
   - Enable **Strict mode** if the app must be installed for the run to pass.
6. Click **Install & Launch** — installs APK, verifies package, launches app, verifies process.
7. Click **Bootstrap Connection** — sets `adb reverse tcp:11369`, verifies listing, optionally bridges WiFi.
8. Badge should now show `READY`.
9. Click **Run Full Validation** — runs end-to-end regression with schema_version 2 artifact.
10. Check per-step table for PASS/SKIP/FAIL with ms timings and detail.
11. Run `npm run evidence:append-usb-ledger` from `bonsai-workspace/src` to append to ledger.

**CLI equivalent (for CI or scripting):**

```powershell
# Basic non-strict run:
npm --prefix "z:\Projects\BonsaiWorkspace\bonsai-workspace\src" run test:android-usb-regression

# Strict run with specific APK:
$env:BONSAI_REQUIRE_APP = "1"
$env:ANDROID_APK_PATH = "C:\path\to\app.apk"
npm --prefix "z:\Projects\BonsaiWorkspace\bonsai-workspace\src" run test:android-usb-regression

# Append to evidence ledger:
npm --prefix "z:\Projects\BonsaiWorkspace\bonsai-workspace\src" run evidence:append-usb-ledger
```

**Environment variables (CLI):**

| Variable | Default | Description |
|---|---|---|
| `ANDROID_SERIAL` | auto | Target device serial |
| `BONSAI_API_PORT` | `11369` | Port for `adb reverse` |
| `ANDROID_PACKAGE` | `com.bonsai.workspace` | Package name |
| `ANDROID_ACTIVITY` | (monkey) | Explicit launch activity |
| `ANDROID_APK_PATH` | (auto) | Path to APK; auto-resolves from build output |
| `BONSAI_REQUIRE_APP` | `0` | `1` = strict mode: fail if not installed/launched |
| `ANDROID_ENABLE_BOOTSTRAP` | `0` | `1` = run WiFi bridge after reverse mapping |
| `ANDROID_WIFI_HOST` | (none) | Device WiFi IP for bridge |
| `ANDROID_WIFI_PORT` | `5555` | WiFi debug port |

**Implemented Desktop controls (ADB):**

- Device discovery with state/model display
- Device readiness state machine (`disconnected` → `unauthorized` → `online` → `ready`)
- APK resolver (auto-discovers from Tauri mobile build outputs)
- Install & Launch orchestrator with per-step results + strict mode
- Connection bootstrap (reverse mapping + optional WiFi bridge) with step verification
- Full regression suite with schema_version 2 artifact
- Shell command execution for diagnostics

ADB resolution:

- Desktop auto-resolves `adb` from common SDK locations when PATH is not configured.
- The resolved adb executable path is shown in Settings (`Android USB Lab`) for diagnostics.

USB regression artifacts:

- `Run USB Regression Suite` appends JSONL evidence to `android-usb-regression-evidence.jsonl` in app data.
- Each record includes step results (discover/model/reverse/launch and optional WiFi bridge steps).
- CLI companion: `npm run test:android-usb-regression` (from `bonsai-workspace/src`) writes `tool_test/android-usb-regression/latest.json`.
- Manual CI companion: workflow `android-usb-regression-manual` runs the same regression on a self-hosted Windows runner and uploads `tool_test/android-usb-regression/latest.json`.
- Manual CI also writes a GitHub Actions Summary table with input parameters and per-step PASS/FAIL details for operator sign-off.

Latest local USB regression evidence snapshot (2026-04-16):

- Command: `npm run test:android-usb-regression` (from `bonsai-workspace/src`)
- Artifact: `tool_test/android-usb-regression/latest.json`
- Artifact SHA256: `6882AC4B5E0B6C834DF2B6E8499837D11776161CD23C2EC54BDEA85626B55FC6`
- Timestamp (artifact): `2026-04-16T12:57:19.566Z`
- Result: `ok=true`
- Device serial: `G8S1KT06151202JN`
- Model check: `KFTRWI`
- API reverse mapping: `tcp:11369 -> tcp:11369`
- Package check: `com.bonsai.workspace` not installed (non-fatal because `BONSAI_REQUIRE_APP` was not set)

Release evidence ledger:

| Date (UTC) | Evidence source | Run reference | Artifact | SHA256 | Device serial | Verdict | Notes |
|---|---|---|---|---|---|---|---|
| 2026-04-16T12:57:19.566Z | Local CLI (`test:android-usb-regression`) | local-shell | `tool_test/android-usb-regression/latest.json` | `6882AC4B5E0B6C834DF2B6E8499837D11776161CD23C2EC54BDEA85626B55FC6` | `G8S1KT06151202JN` | PASS | Reverse mapping verified (`tcp:11369`); package launch intentionally non-fatal because `BONSAI_REQUIRE_APP` unset. |

| 2026-04-16T13:17:53.111Z | Local CLI (test:android-usb-regression) | local-shell | `tool_test/android-usb-regression/latest.json` | `4A6E7D01AC5C84BA74081B4AA60B6154209736AB69708C0FC999EF0397010F35` | `G8S1KT06151202JN` | PASS | reverse mapping target api port 11369; Package com.bonsai.workspace is not installed; set BONSAI_REQUIRE_APP=1 to make this mandatory. |

| 2026-04-16T14:21:47.817Z | Local CLI (test:android-usb-regression) | local-shell | `tool_test/android-usb-regression/latest.json` | `028F380C302859FB9A738ABCB7E02A61CF8FA0E27CBD2D13EAD402F1A324F4FD` | `G8S1KT06151202JN` | PASS | reverse mapping target api port 11369; Package com.bonsai.workspace is not installed; set BONSAI_REQUIRE_APP=1 to make this mandatory. |

| 2026-04-16T14:33:27.407Z | Local CLI (test:android-usb-regression) | local-shell | `tool_test/android-usb-regression/latest.json` | `C960F577E4FFBBE97541D34BFF449E6BE7813999594AD82241C35E7150828465` | `G8S1KT06151202JN` | PASS | api port 11369; non-strict; Package com.bonsai.workspace is not installed; set BONSAI_REQUIRE_APP=1 to make this mandatory. |

| 2026-04-16T14:37:37.840Z | Local CLI (test:android-usb-regression) | local-shell | `tool_test/android-usb-regression/latest.json` | `0692E029FDF9298799FD2B5B0047D0282E3C6522050DC228F332E91C326E63FE` | `G8S1KT06151202JN` | PASS | api port 11369; non-strict; Package com.bonsai.workspace is not installed; set BONSAI_REQUIRE_APP=1 to make this mandatory. |

| 2026-04-17T04:39:59.929Z | Local CLI (test:android-usb-regression) | local-shell | `tool_test/android-usb-regression/latest.json` | `D22298182F45F969A33CA2F780A00E0D91AE0A9A8130B6B8E55AA830D543D80D` | `G8S1KT06151202JN` | PASS | api port 11369; non-strict; Package com.bonsai.workspace is not installed; set BONSAI_REQUIRE_APP=1 to make this mandatory. |

Ledger update rule:

- Add one row for each manual CI hardware run and each local sign-off run.
- For manual CI runs, use GitHub run URL in Run reference and copy SHA256 from the run summary.
- Do not overwrite historical rows; append new rows only.
- Local helper command: run `npm --prefix "z:\Projects\BonsaiWorkspace\bonsai-workspace\src" run evidence:append-usb-ledger` after each successful USB regression artifact update.

### CI and automation completion

- [x] API smoke included in a required CI lane.
- [x] VSCode extension build/tests included as required checks.
- [x] Android build lane present at smoke/check level.
- [x] Additional OS added to nightly soak coverage.

### Exit criteria for release-grade confidence

1. Required checks green for desktop app, API smoke, and VSCode extension.
2. Android real-device validation documented with command/log evidence.
3. Pairing and reconnect succeed without manual workarounds.
4. No known spec/runtime contract mismatches remain in this document.







