# 🌿 Bonsai Workspace

**A local-first, AI-powered code editor built on Tauri 2 + Svelte + llama.cpp.**

All AI inference runs on-device — no cloud API keys, no data leaves your machine.

---

## Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | ≥ 1.77  | `curl https://sh.rustup.rs -sSf \| sh` |
| Node.js | ≥ 20 | https://nodejs.org |
| Tauri CLI | 2.x | `cargo install tauri-cli --version "^2"` |
| System deps | — | See below |

### System dependencies

**macOS**
```bash
xcode-select --install
```

**Ubuntu / Debian**
```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget \
  libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
  libasound2-dev libsqlite3-dev
```

**Windows**
- Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) with "Desktop development with C++"
- Install [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

---

## Build & Run

> **Important — project layout:** The frontend (`package.json`, `vite.config.ts`) lives
> inside `src/`, not at the workspace root. All `npm` commands must be run from there.
> `cargo tauri dev` must be run from `src-tauri/` (or the workspace root — Tauri finds
> `src-tauri/` automatically).

```bash
# 1. Install frontend dependencies  (must be run from src/)
cd src
npm install
cd ..

# 2. Launch in dev mode
#    Tauri automatically starts `npm --prefix ../src run dev` (configured in tauri.conf.json)
cd src-tauri
cargo tauri dev
```

### First-time AI setup — sidecar binaries

Bonsai expects two sidecar binaries. During development they are **optional** — the app
runs without AI and shows a timeout warning in the console after 30 s.

#### Step 1 — Download prebuilt binaries

| Binary | Source |
|--------|--------|
| `llama-server` | https://github.com/ggerganov/llama.cpp/releases — grab the asset matching your platform (`llama-*-bin-*.zip`), extract, find `llama-server` / `llama-server.exe` |
| `whisper-server` | https://github.com/ggerganov/whisper.cpp/releases — grab `whisper-*`, extract, find `server` / `server.exe` and rename it `whisper-server` |

#### Step 2 — Place and rename to match Tauri's platform suffix

Tauri appends a target triple to sidecar names at runtime. Rename the binary you
downloaded to match your platform exactly, then place it in `src-tauri/binaries/`:

```
src-tauri/binaries/
├── llama-server-x86_64-pc-windows-msvc.exe    ← Windows x64
├── llama-server-x86_64-apple-darwin            ← macOS Intel
├── llama-server-aarch64-apple-darwin           ← macOS Apple Silicon
├── llama-server-x86_64-unknown-linux-gnu       ← Linux x64
├── whisper-server-x86_64-pc-windows-msvc.exe
├── whisper-server-x86_64-apple-darwin
├── whisper-server-aarch64-apple-darwin
└── whisper-server-x86_64-unknown-linux-gnu
```

Only the binary for **your** platform is needed. The others are for bundled distribution.

To find the exact triple Rust uses on your machine:
```bash
rustc -vV | grep host
# host: x86_64-pc-windows-msvc
```

#### Step 3 — Download a language model

Open Bonsai → Settings → click the **⬇** button next to a model to download it
automatically, **or** click **Import Local GGUF** to load a file you already have.

Models land in `%APPDATA%\bonsai-workspace\models\` (Windows) or
`~/Library/Application Support/bonsai-workspace/models/` (macOS).

#### Step 4 — Download the Whisper model

```bash
# Download the base English model (~150 MB)
curl -L https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin \
     -o "%APPDATA%\bonsai-workspace\models\ggml-base.en.bin"
```

Or use Settings → **⬇ Download Whisper**.

---

## Directory Structure

```
bonsai-workspace/
├── src-tauri/                  # Rust backend (Tauri 2)
│   ├── src/
│   │   ├── lib.rs              # App setup, state management
│   │   ├── main.rs             # Entry point (calls lib::run)
│   │   ├── commands.rs         # All #[tauri::command] handlers
│   │   ├── sidecar_manager.rs  # llama.cpp + whisper.cpp orchestration
│   │   ├── wal.rs              # SQLite write-ahead log
│   │   └── action_parser.rs    # AgentAction JSON schema
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── capabilities/default.json
│
└── src/                        # Svelte frontend
    ├── lib/
    │   ├── stores/
    │   │   ├── workspace.ts    # Current open folder + fileTreeRefresh trigger
    │   │   ├── chat.ts         # Messages, permission cards, isThinking
    │   │   ├── diff.ts         # Unified diff parsing + current diff state
    │   │   ├── terminal.ts     # Terminal panel visibility
    │   │   └── openFile.ts     # Cross-component file-open signal (store pattern)
    │   ├── utils/
    │   │   └── monaco.ts       # Editor factory, language detection, diff decorations
    │   └── components/
    │       ├── FileTree.svelte       # File explorer, workspace opener
    │       ├── MonacoEditor.svelte   # Code editor + inline diff widgets
    │       ├── ChatPanel.svelte      # AI chat, voice input, permission cards
    │       ├── StatusBar.svelte      # Token speed, git branch, memory warning
    │       ├── CommandPalette.svelte # Ctrl+K fuzzy command launcher
    │       ├── TerminalPanel.svelte  # xterm.js + PTY shell
    │       ├── SettingsPanel.svelte  # Model management, hardware info
    │       ├── DownloadProgress.svelte
    │       ├── PermissionCard.svelte
    │       ├── TemplateSelector.svelte
    │       └── CodeReviewPanel.svelte
    ├── App.svelte              # Root layout, theme system
    ├── main.ts                 # Tauri event listeners, Svelte mount
    ├── vite.config.ts
    ├── tsconfig.json
    └── package.json
```

---

## Key Architecture Decisions (vs original blueprint)

### Bugs fixed from original

| # | Original bug | Fix |
|---|-------------|-----|
| 1 | `createApp()` — Vue API used in Svelte project | `new App({ target })` |
| 2 | `emit_all()` — doesn't exist in Tauri v2 | `app_handle.emit()` |
| 3 | Borrow-after-move in `process_loop` | Destructure before move; tx consumed after result |
| 4 | `WAL::new` async, called sync in setup | `rt.block_on(WAL::new(...))` |
| 5 | `export let openFile` reassigned in onMount | `openFileRequest` store — FileTree writes, Editor subscribes |
| 6 | `refreshFileTree` exported from Svelte file | `fileTreeRefresh` writable store |
| 7 | `$: if (get(store))` not reactive | `$: $store, fn()` — correct reactive statement |
| 8 | `{@html}` inside class attr — syntax error | Proper conditional block |
| 9 | `bind:this` to get component function | Store-based signal pattern |
| 10 | `Patch::from_hunks()` — not in diffy 0.3 | Manual hunk extraction + string reconstruction |
| 11 | PTY `write_all()` on PtyPair.master | Separate `take_writer()` + `pty_writer` state |
| 12 | `"api-all"` feature — doesn't exist | Removed; `"devtools"` only |
| 13 | `devPath`/`distDir` — wrong Tauri v2 keys | `devUrl`/`frontendDist` |
| 14 | CSP null | Proper CSP in both tauri.conf.json and index.html |
| 15 | No `Arc` import in voice_transcribe | Fixed imports |
| 16 | Voice WavWriter moved into closure, used after | `Arc<Mutex<Vec<i16>>>` shared PCM buffer |
| 17 | Path traversal in FileCreate | `..` + `is_absolute()` guard |
| 18 | `listen` wrong import path in StatusBar | `@tauri-apps/api/event` |
| 19 | Dialog API wrong for Tauri v2 | `app_handle.dialog().file().blocking_pick_folder()` |
| 20+ | Various UX issues | Error states, loading indicators, empty states, a11y |

### Cross-component file opening (store pattern)

```
FileTree clicks file
  → writes path to openFileRequest store
    → MonacoEditor subscribes
      → calls openFile(path) internally
```

No `bind:this`, no exported functions from Svelte files, no prop threading.

### Diff application

`accept_diff_hunk` manually splits the unified diff at `@@` markers, extracts
the target hunk block, prepends the file header, then applies that minimal
patch via `diffy::apply`. This avoids the `Patch::from_hunks()` API that
doesn't exist in diffy 0.3.

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+K / ⌘K | Command Palette |
| Enter (chat) | Send message |
| Shift+Enter | Newline in chat |

---

## Production Build

```bash
cargo tauri build
```

Outputs installers to `src-tauri/target/release/bundle/`.

---

## Agent Smoke Testing (API + ScreenShare/RemoteAccess)

Use the built-in HTTP API to validate that Bonsai is usable by any agent tooling
without UI-specific hooks.

### What this smoke test verifies

- API liveness and version endpoints (`/health`, `/api/version`)
- OpenAI/Ollama-compatible model listing (`/v1/models`, `/api/tags`)
- Remote access lifecycle:
  - start session (`/remote/session/start`)
  - offer negotiation contract (`/remote/session/offer`)
  - frame capture (`/remote/frame`)
  - SSE frame stream (`/remote/stream`)
  - remote input event (`/remote/input`)
  - stop session (`/remote/session/stop`)
- Scripted UI HITL flow (Playwright):
  - send chat prompt
  - render permission card
  - approve path (`✓ Approve`)
  - deny path (`✕ Deny`)

### Run it

1. Start Bonsai (so the built-in API server is running)
2. Run from `src/`:

```bash
npm run test:agent-api
```

Runner mode toggles:

```bash
# API/remote only
npm run test:agent-api:remote-only

# UI HITL only (requires frontend dev server at http://localhost:1420)
npm run test:agent-ui-hitl

# Visible real-time UI demo (streaming + HITL approve/deny shown on screen)
npm run test:agent-ui-live

# One-command visible demo with auto dev-server connect/start + real-time HITL flow
npm run test:agent-ui-live-orchestrated

# Independent desktop app-window live mode (opens Bonsai via Tauri, not a browser)
npm run demo:app-window-live

# One-command orchestration (start stack, wait, run all tests, teardown)
npm run test:agent-orchestrated

# CI-friendly routing regressions (starts dev server, runs Rust routing tests + UI HITL, then stops server)
npm run test:agent-routing-ci

# Optional: increase scripted UI wait budget on slower CI agents
# Recommended range: 30000-90000 ms (default 45000)
BONSAI_UI_STEP_TIMEOUT_MS=60000 npm run test:agent-routing-ci
```

Optional custom endpoint:

```bash
BONSAI_API_BASE=http://127.0.0.1:11369 npm run test:agent-api
```

If the API is not running, the script exits with actionable guidance.
