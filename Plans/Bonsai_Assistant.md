Bonsai Assistant ("Bonsai Buddy") — Implementation Plan
Context
The user wants a next-generation personal AI assistant integrated into Bonsai Workspace that:

Runs as a standalone floating window, independent of the main workspace (survives workspace close)
Can perform any local task: weather, file search, web scraping, email, charts, shell commands, system info
Features an animated avatar with TTS lip-sync (mouth movement synced to speech)
Includes a full management system: assistant profiles, avatar library, backup/restore
Works on both desktop (Windows primary) and Android
This is a Phase 1–4 phased implementation. The plan is complete end-to-end.

Architecture Decision: Second Tauri Webview Window
The assistant runs as a second named Tauri webview window ("assistant") within the same Tauri process — sharing AppState, ModelOrchestrator, and the SQLite pool. A separate OS process would require duplicating model loading, the inference engine, and the database.

A custom on_window_event hook in lib.rs keeps the process alive when the main workspace is closed while the assistant is open:

app.on_window_event(|window, event| {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        if window.label() == "main" {
            if window.app_handle().get_webview_window("assistant").is_some() {
                window.hide().unwrap();
                api.prevent_close();
            }
        }
    }
});
Assistant window in tauri.conf.json:

{
  "label": "assistant", "title": "Bonsai Buddy",
  "width": 420, "height": 680, "minWidth": 340, "minHeight": 480,
  "resizable": true, "visible": false, "decorations": true,
  "alwaysOnTop": true, "url": "assistant.html"
}
System tray via tauri-plugin-tray: left-click toggles window; context menu has "Show Bonsai Buddy" and "Quit".

TTS Engine: Piper TTS (sidecar binary)
Decision: Piper TTS — C++ binary (~20MB), ONNX runtime, natural speech, phoneme timing JSON output.

Why not alternatives:

System TTS (SAPI/AVSpeech): word-level timing only, platform-specific FFI, no cross-platform phoneme data
kokoro-tts: requires Python + torch (~2GB overhead)
espeak-ng: robotic quality unacceptable for avatar companion
Piper with --json_input writes raw WAV to a temp file and emits phoneme timing JSON to stdout. Playback via rodio crate. Download follows the same bootstrap pattern as llama-server and whisper-server. Download URL: https://github.com/rhasspy/piper/releases/latest → piper_windows_amd64.zip.

Avatar System: Inline SVG with Svelte RAF Loop
Decision: Inline SVG — Each avatar is a structured SVG file with 14 named mouth-shape paths (data-viseme="0" through "13"), CSS-animated eyes (blink), and body (breathe). Svelte drives mouth shape switching via requestAnimationFrame.

Why not Lottie: requires lottie-web (~250KB), pre-baked sequences can't accept real-time viseme injection. Why not Three.js: WebGL (excluded by constraint).

14-Viseme set (Preston Blair standard):

ID	Shape	Phonemes
0	Silence	pause, SIL
1	AE/AH	AE, AH
2	EH/ER	EH, ER
3	IY	IY, IH
4	AW/AO	AW, AO, AA
5	OW/UH	OW, UH
6	UW	UW, OY
7	MBP	M, B, P
8	F/V	F, V
9	TH	TH, DH
10	TDS	T, D, S, Z
11	CH	CH, SH, ZH, JH
12	N	N, NG, L
13	R	R
Lip-sync flow:

submit_assistant_chat calls tts_manager.speak(text) when speak_response=true
Rust emits tts-visemes to the "assistant" window: { duration_ms, events: [{viseme_id, start_ms}] }
AssistantAvatar.svelte records speechStartMs = performance.now()
requestAnimationFrame loop walks events array by elapsed time → updates SVG mouth shape
Data Model (SQLite — same pool as WAL)
assistant_profiles
CREATE TABLE IF NOT EXISTS assistant_profiles (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL DEFAULT 'Bonsai Buddy',
    persona_id      TEXT REFERENCES personas(id) ON DELETE SET NULL,
    avatar_id       TEXT REFERENCES avatar_assets(id) ON DELETE SET NULL,
    tts_voice       TEXT NOT NULL DEFAULT 'en_US-amy-medium',
    tts_speed       REAL NOT NULL DEFAULT 1.0,
    tts_pitch       REAL NOT NULL DEFAULT 1.0,
    tts_enabled     INTEGER NOT NULL DEFAULT 1,
    wake_word       TEXT,
    tool_permissions TEXT NOT NULL DEFAULT '{}',
    system_prompt   TEXT NOT NULL DEFAULT 'You are Bonsai Buddy, a helpful personal AI assistant.',
    model_id        TEXT,
    is_active       INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);
avatar_assets
CREATE TABLE IF NOT EXISTS avatar_assets (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    asset_type   TEXT NOT NULL CHECK(asset_type IN ('svg_builtin','svg_custom','photo')),
    asset_data   TEXT,      -- SVG text or base64 (small assets)
    file_path    TEXT,      -- disk path (large assets)
    thumbnail_svg TEXT,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL
);
assistant_sessions + assistant_messages
CREATE TABLE IF NOT EXISTS assistant_sessions (
    id TEXT PRIMARY KEY, profile_id TEXT REFERENCES assistant_profiles(id) ON DELETE CASCADE,
    title TEXT NOT NULL DEFAULT 'New conversation', created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS assistant_messages (
    id TEXT PRIMARY KEY, session_id TEXT NOT NULL REFERENCES assistant_sessions(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK(role IN ('user','assistant','tool')), content TEXT NOT NULL,
    tool_name TEXT, tool_result TEXT, tts_synthesized INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL
);
backup_registry
CREATE TABLE IF NOT EXISTS backup_registry (
    id TEXT PRIMARY KEY, filename TEXT NOT NULL, file_path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL, includes TEXT NOT NULL, created_at INTEGER NOT NULL
);
New Rust Modules
File	Purpose
src-tauri/src/assistant_store.rs	SQLite CRUD: profiles, avatars, sessions, messages, backup registry
src-tauri/src/tts_manager.rs	Piper sidecar lifecycle, WAV synthesis, viseme timeline, rodio playback
src-tauri/src/assistant_tools.rs	Tool registry: fetch_url, get_weather, render_chart, find_files, send_email, get_datetime, get_system_stats, open_url
src-tauri/src/assistant_manager.rs	run_assistant_turn() — ReAct tool loop + TTS trigger
src-tauri/src/assistant_backup.rs	ZIP export/import using existing zip crate
src-tauri/src/assistant_commands.rs	All new Tauri commands (separate from the already-large commands.rs)
Key Rust Signatures
// assistant_store.rs
pub struct AssistantStore { pool: SqlitePool }
impl AssistantStore {
    pub async fn new(pool: SqlitePool) -> Result<Self>;
    pub async fn list_profiles(&self) -> Result<Vec<AssistantProfile>>;
    pub async fn get_active_profile(&self) -> Result<Option<AssistantProfile>>;
    pub async fn upsert_profile(&self, p: AssistantProfile) -> Result<AssistantProfile>;
    pub async fn delete_profile(&self, id: &str) -> Result<()>;
    pub async fn set_active_profile(&self, id: &str) -> Result<()>;
    pub async fn list_avatars(&self) -> Result<Vec<AvatarAsset>>;
    pub async fn upsert_avatar(&self, a: AvatarAsset) -> Result<AvatarAsset>;
    pub async fn delete_avatar(&self, id: &str) -> Result<()>;
    pub async fn list_sessions(&self, profile_id: Option<&str>, limit: i64) -> Result<Vec<AssistantSession>>;
    pub async fn create_session(&self, profile_id: Option<&str>, title: &str) -> Result<AssistantSession>;
    pub async fn save_message(&self, msg: AssistantMessage) -> Result<AssistantMessage>;
    pub async fn register_backup(&self, filename: &str, path: &str, size: i64, includes: &[&str]) -> Result<()>;
}

// tts_manager.rs
pub struct SynthResult { pub wav_bytes: Vec<u8>, pub duration_ms: u32, pub viseme_timeline: Vec<VisemeEvent> }
pub struct TtsManager {
    app: AppHandle, voice: Arc<Mutex<String>>, speed: Arc<Mutex<f32>>,
    client: reqwest::Client, process: Mutex<Option<Child>>,
    _stream: Mutex<Option<rodio::OutputStream>>, stream_handle: Mutex<Option<rodio::OutputStreamHandle>>
}
impl TtsManager {
    pub fn new(app: &AppHandle) -> Self;
    pub async fn synthesize(&self, text: &str) -> Result<SynthResult, String>;
    pub async fn speak(&self, app: &AppHandle, text: &str) -> Result<(), String>;
    pub fn stop(&self);
    pub fn set_voice(&self, voice: &str);
    pub fn set_speed(&self, speed: f32);
    fn ensure_running(&self) -> Result<(), String>;
}

// assistant_manager.rs
pub async fn run_assistant_turn(
    messages: Vec<serde_json::Value>, profile: &AssistantProfile,
    store: &AssistantStore, orchestrator: &ModelOrchestrator, tts: &TtsManager,
    app: &AppHandle, cancel_flag: Arc<AtomicBool>,
    stream_tx: Option<mpsc::UnboundedSender<String>>,
) -> Result<AssistantTurn, String>;
AppState additions (lib.rs)
pub assistant_store:  Arc<assistant_store::AssistantStore>,
pub tts_manager:      Arc<tts_manager::TtsManager>,
pub assistant_cancel: Arc<AtomicBool>,
New Cargo.toml Dependencies
tauri-plugin-tray         = "2"
tauri-plugin-window-state = "2"
rodio   = { version = "0.17", default-features = false, features = ["wav"] }
lettre  = { version = "0.11", default-features = false, features = ["smtp-transport","rustls-tls","builder"] }
New Tauri Commands (assistant_commands.rs)
list_assistant_profiles, get_active_assistant_profile, upsert_assistant_profile,
delete_assistant_profile, set_active_assistant_profile,
list_avatar_assets, upsert_avatar_asset, delete_avatar_asset, import_avatar_from_file,
speak_text, stop_tts, list_tts_voices, download_tts_voice,
list_assistant_sessions, load_assistant_session, delete_assistant_session,
submit_assistant_chat, stop_assistant_chat,
export_assistant_backup, import_assistant_backup, list_assistant_backups,
toggle_assistant_window, set_assistant_always_on_top,
render_assistant_chart
Frontend Architecture
New Entry Point (Multi-Page Vite Build)
src/assistant.html — mirror of index.html, loads assistant-main.ts
src/assistant-main.ts — mounts AssistantApp.svelte (NO Monaco, xterm, OpenCV)
src/AssistantApp.svelte — root, manages overlay booleans
vite.config.ts change:

build: { rollupOptions: { input: { main: 'index.html', assistant: 'assistant.html' } } }
New Svelte Components
File	Purpose
src/lib/components/assistant/BonsaiAssistant.svelte	Main layout: toolbar + avatar + chips + messages + input
src/lib/components/assistant/AssistantAvatar.svelte	SVG rendering + RAF lip-sync + idle animations
src/lib/components/assistant/AssistantToolbar.svelte	Top bar: profile name, pin toggle, settings, minimize
src/lib/components/assistant/AssistantInputBar.svelte	Text input + voice record + TTS toggle + send
src/lib/components/assistant/AssistantMessageList.svelte	Scrollable history
src/lib/components/assistant/AssistantMessage.svelte	Bubble: user (right, accent) / assistant (left, bg2)
src/lib/components/assistant/InlineToolResult.svelte	Renders: weather card, SVG chart, file list, plain text
src/lib/components/assistant/QuickActionChips.svelte	Horizontal chips: Weather, Time, Files, System, Web, New Chat
src/lib/components/assistant/AvatarPicker.svelte	Grid of built-in avatars + import SVG
src/lib/components/assistant/ProfileManager.svelte	CRUD: name, persona, TTS voice/speed, tool permissions
src/lib/components/assistant/AssistantSettings.svelte	Always-on-top, TTS toggle, SMTP config, tool toggles
src/lib/components/assistant/BackupManager.svelte	Export/import UI with backup list
src/lib/components/assistant/AssistantSessionHistory.svelte	Session browser with search
New Svelte Stores
File	Key Exports
src/lib/stores/assistant.ts	activeProfile, allProfiles, avatarAssets, currentAvatarSvg, assistantMessages, isAssistantThinking, isSpeaking, currentVisemeId, sendAssistantMessage(), initAssistantStores()
src/lib/stores/assistantSessions.ts	currentSessionId, loadSessions(), createSession()
src/lib/stores/tts.ts	ttsVoices, currentVoice, ttsDownloadProgress
Tauri Events (Rust → "assistant" window only)
Event	Payload	Purpose
token-stream-assistant	string	Streaming tokens (isolated from workspace token-stream)
tts-visemes	{duration_ms, events:[{viseme_id,start_ms}]}	Lip-sync timeline
tts-started	{duration_ms}	Audio started
tts-done	{}	Audio finished
tts-error	string	TTS failure
assistant-tool-start	{tool, args}	Tool began
assistant-tool-done	{tool, result}	Tool result
tts-download-progress	{pct, voice}	Voice model downloading
Desktop GUI Layout
┌─────────────────────────────────────────┐  420px wide
│  🌿 Bonsai Buddy  [Amy ▾] [📌][⚙][×]  │  44px toolbar
├─────────────────────────────────────────┤
│                                         │
│         [Animated SVG Avatar]           │  180px avatar panel
│        (blink, breathe, lip sync)       │
│                                         │
├─────────────────────────────────────────┤
│ [🌤Weather][🕐Time][🔍Files][💻Stats]→  │  44px quick chips
├─────────────────────────────────────────┤
│  You: what's the weather?              ↑│
│  ┌──────────────────────────────────┐   │
│  │ 🌤 12°C · Partly cloudy · W 14  │   │  flex:1 scrollable
│  └──────────────────────────────────┘   │
│  You: chart my RAM usage               ↓│
├─────────────────────────────────────────┤
│ [🎤] [ Type a message...  ] [➤] [🔊]   │  52px input bar
└─────────────────────────────────────────┘
Message bubbles:

User: right-aligned, background: var(--accent), border-radius: 16px 16px 4px 16px
Assistant: left-aligned, background: var(--bg2), border: 1px solid var(--border), border-radius: 4px 16px 16px 16px
Tool cards: collapsible, background: var(--bg), dimmed text
Android Design
Same SVG avatar renders identically in Android WebView. TTS falls back to Web Speech API:

const isMobile = /android/i.test(navigator.userAgent);
async function speak(text: string) {
  if (isMobile) {
    const u = new SpeechSynthesisUtterance(text);
    u.onboundary = (e) => { currentVisemeId = (e.charIndex % 7) + 1; };
    u.onend = () => { currentVisemeId = 0; };
    speechSynthesis.speak(u);
  } else {
    await invoke('speak_text', { text });
  }
}
MobileLayout.svelte gains a "Buddy" tab showing AssistantMobile.svelte.

Backup ZIP Format
bonsai-buddy-backup-{date}.zip
├── manifest.json       # {version, created_at, app_version}
├── profiles/{id}.json
├── avatars/{id}.json + {id}.svg
└── sessions/{id}.json  # session + all messages
Auto-backup on app close: keep last 5, stored in {app_data}/backups/. Uses existing zip crate.

Modified Existing Files
File	Changes
src-tauri/src/lib.rs	Add 6 new mods; add 3 fields to AppState; window close handler; register all new commands; add tray + window-state plugins
src-tauri/Cargo.toml	Add tray, window-state, rodio, lettre
src-tauri/tauri.conf.json	Add assistant window; tray config; window-state plugin
src-tauri/src/bootstrap.rs	Add piper_exe(), piper_model_path(), tts_ready(), Piper download
src/App.svelte	Add "🌿 Buddy" toolbar button + Ctrl+Shift+B shortcut
src/lib/components/MobileLayout.svelte	Add Buddy tab
src/vite.config.ts	Multi-page build input
Phased Implementation Order
Phase 1 — Foundation (floating window + chat, no TTS/avatar)
tauri.conf.json — add assistant window config
Cargo.toml — add tray + window-state
lib.rs — mods, AppState fields, close handler, command registration, tray setup
assistant_store.rs — complete SQLite CRUD (all 4 tables + seeding)
assistant_commands.rs — CRUD commands + toggle_assistant_window + submit_assistant_chat (speak_response ignored for now)
vite.config.ts — multi-page build
assistant.html + assistant-main.ts + AssistantApp.svelte
stores/assistant.ts — complete store with sendAssistantMessage
BonsaiAssistant.svelte, AssistantInputBar.svelte, AssistantMessageList.svelte, AssistantMessage.svelte (placeholder div where avatar goes)
App.svelte — Buddy button + keyboard shortcut
cargo check + npm run build
Phase 2 — Capability Engine (tools)
assistant_tools.rs — fetch_url, get_weather (open-meteo.com, no API key), render_chart (SVG), find_files, get_datetime, get_system_stats, open_url
assistant_manager.rs — run_assistant_turn() ReAct loop
Update submit_assistant_chat to go through run_assistant_turn
assistant_commands.rs — add render_assistant_chart
Cargo.toml — add lettre; update assistant_commands.rs with send_email tool + SMTP config
QuickActionChips.svelte, InlineToolResult.svelte, AssistantSettings.svelte
cargo check + npm run build
Phase 3 — TTS + Lip Sync + Avatar
tts_manager.rs — complete Piper sidecar + rodio playback + viseme extraction
bootstrap.rs — Piper binary download
lib.rs — add tts_manager to AppState
Cargo.toml — add rodio
assistant_commands.rs — speak_text, stop_tts, list_tts_voices, download_tts_voice
AssistantAvatar.svelte — inline SVG + RAF lip-sync + CSS blink/breathe
stores/tts.ts
AvatarPicker.svelte, ProfileManager.svelte
4–6 built-in avatar SVG files → src-tauri/icons/avatars/*.svg
Android Web Speech API fallback in AssistantAvatar.svelte
cargo check + npm run build
Phase 4 — Management, Backup, Polish
assistant_backup.rs — ZIP export/import
assistant_commands.rs — backup commands
BackupManager.svelte, AssistantSessionHistory.svelte
Auto-backup on close (keep last 5)
Tray menu polish
set_assistant_always_on_top + pin button in toolbar
Window position persistence (window-state plugin)
MobileLayout.svelte — Buddy tab + AssistantMobile.svelte
Piper voice downloader UI in AssistantSettings
cargo check + npm run build
Verification
cargo check after each Phase 1/2/3/4 Rust changes
npm run build after each Phase frontend changes
Phase 1: Tray icon appears → click shows assistant window; type message → AI responds; close workspace → assistant stays open
Phase 2: "what's the weather?" → weather card; "bar chart of [data]" → inline SVG; "find files named *.rs" → file list
Phase 3: AI response is spoken; avatar mouth moves in sync; idle blink/breathe run; swap avatar mid-session works; Android uses Web Speech API
Phase 4: Export → ZIP created in file picker location; import on fresh install → data restored; 5 auto-backups rotate in {app_data}/backups/