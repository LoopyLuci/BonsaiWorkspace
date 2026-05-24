#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]

mod agent;
mod agent_host;
mod agents;
mod features;
mod action_parser;
mod error;
mod memory_store;
mod context_builder;
mod skill_executor;
mod tool_health;
mod mcp_bridge;
mod assistant_audit_log;
mod assistant_backup;
mod assistant_commands;
mod assistant_manager;
mod assistant_metrics;
mod assistant_policy;
mod assistant_store;
mod assistant_tools;
mod tool_cache;
mod tool_core;
mod tool_selector;
mod avatar_validator;
mod secrets_store;
mod tts_manager;
mod agent_connect;
mod agent_store;
mod api_server;
mod buddy_api_server;
mod bootstrap;
mod chat_sessions;
mod cluster_orchestrator;
mod commands;
mod config;
mod inference_mode;
mod model_data;
mod model_data_store;
mod model_data_generator;
mod model_orchestrator;
mod model_registry;
pub mod rag_store;
mod remote;
mod remote_input;
mod sidecar_manager;
mod sidecar_supervisor;
mod swarm_orchestrator;
mod task_queue;
mod tools;
mod user_skills;
mod wal;
mod ws_router;

/// Write `content` to `path` atomically: write to a `.tmp` sibling then rename.
/// Ensures the file is either fully written or unchanged on crash.
pub fn atomic_write(path: &std::path::Path, content: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(content)?;
        f.flush()?;
    }
    std::fs::rename(&tmp, path)
}

use std::collections::HashMap;
use std::sync::{
    atomic::AtomicBool,
    Arc,
    Mutex as StdMutex,
    OnceLock,
};

// Keeps the non-blocking tracing writer flushed for the lifetime of the process.
static LOG_GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();
use tauri::Emitter;
use tauri::Manager;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use tauri::{PhysicalPosition, PhysicalSize, Position, Size, Window};
use tokio::sync::Mutex;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub struct PtySession {
    pub writer: Box<dyn std::io::Write + Send>,
    pub master: Box<dyn portable_pty::MasterPty + Send>,
}

#[cfg(any(target_os = "android", target_os = "ios"))]
pub struct PtySession;

#[derive(Clone)]
pub struct AppState {
    pub orchestrator:     Arc<model_orchestrator::ModelOrchestrator>,
    pub whisper:          Arc<sidecar_manager::WhisperManager>,
    pub wal:              Arc<wal::WAL>,
    pub chat_sessions:    Arc<chat_sessions::ChatSessionStore>,
    pub pty_sessions:     Arc<Mutex<std::collections::HashMap<String, PtySession>>>,
    /// Set to `true` to cancel an in-progress bootstrap download.
    pub bootstrap_cancel: Arc<AtomicBool>,
    /// Set to `true` to cancel in-flight chat generation.
    pub chat_cancel:      Arc<AtomicBool>,
    /// Set to `true` to stop active voice capture.
    pub voice_cancel:     Arc<AtomicBool>,
    /// Agent Connect sessions and typed event timelines.
    pub agent_connect:    Arc<StdMutex<agent_connect::AgentConnectHub>>,
    /// WebSocket connection registry (Android app + VSCode extension relay).
    pub ws_router:        Arc<ws_router::WsRouter>,
    /// One-time pairing token displayed as a QR code in Settings.
    pub pair_token:       String,
    /// Agent persona/config store (shared SQLite pool).
    pub agent_store:      Arc<agent_store::AgentStore>,
    /// Swarm coordinator — routes leader/worker inference.
    pub swarm_orchestrator: Arc<swarm_orchestrator::SwarmOrchestrator>,
    /// Per-run per-slot cancel flags for in-flight swarm agents.
    pub swarm_cancels:    Arc<StdMutex<HashMap<String, Vec<Arc<AtomicBool>>>>>,
    /// Managed API server runtime for controlled restart/shutdown.
    pub api_server:       Arc<Mutex<Option<api_server::ApiServerHandle>>>,
    /// Multi-device clustering coordinator and scheduler.
    pub cluster_orchestrator: Arc<Mutex<cluster_orchestrator::ClusterOrchestrator>>,
    /// Tool policy engine — evaluates allow/deny/confirm for every assistant tool call.
    pub policy_engine:     Arc<assistant_policy::PolicyEngine>,
    /// Confirmation gate — single-use tokens for high-risk action approval.
    pub confirmation_gate: Arc<assistant_policy::ConfirmationGate>,
    /// Rotating structured audit log — every tool attempt recorded.
    pub audit_log:         Arc<assistant_audit_log::AuditLog>,
    /// OS keychain abstraction — SMTP credentials and future secrets.
    pub secrets_store:     Arc<secrets_store::SecretsStore>,
    /// Assistant SQLite CRUD store (profiles, avatars, sessions, messages).
    pub assistant_store:   Arc<assistant_store::AssistantStore>,
    /// Set to `true` to cancel an in-progress assistant inference turn.
    pub assistant_cancel:  Arc<std::sync::atomic::AtomicBool>,
    /// Structured performance + error counters for the assistant.
    pub asst_metrics:      Arc<assistant_metrics::AssistantMetrics>,
    /// Piper TTS sidecar — speech synthesis + rodio playback.
    pub tts_manager:       Arc<tts_manager::TtsManager>,
    /// User-defined skills store (SQLite-backed, hot-reloadable into tool registry).
    pub user_skill_store:  Arc<user_skills::UserSkillStore>,
    /// MCP server lifecycle manager.
    pub mcp_manager:       Arc<mcp_bridge::McpManager>,
    /// Buddy API server handle (port 11420). None if startup failed.
    pub buddy_api_server:  Arc<Mutex<Option<buddy_api_server::BuddyApiHandle>>>,
    /// Resolved port for the Buddy API (0 if unavailable).
    pub buddy_api_port:    u16,
    /// Rich, persistent metadata for every model (local and cloud).
    pub model_data_store:  Arc<model_data_store::ModelDataStore>,
    /// Shared inference task queue with fairness/resource gating.
    pub task_queue:       Arc<task_queue::TaskQueue>,
    /// Pluggable agent registry with built-in CodeWriter and CodeReviewer.
    pub agent_host:       Arc<agent_host::AgentHost>,
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn restore_main_window_state(app: &tauri::AppHandle, cfg: &config::AppConfig) {
    if let Some(main) = app.get_webview_window("main") {
        if let (Some(w), Some(h)) = (cfg.main_window_width, cfg.main_window_height) {
            let _ = main.set_size(Size::Physical(PhysicalSize::new(w, h)));
        }
        if let (Some(x), Some(y)) = (cfg.main_window_x, cfg.main_window_y) {
            let _ = main.set_position(Position::Physical(PhysicalPosition::new(x, y)));
        }
        ensure_main_window_visible(&main);
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn ensure_main_window_visible(main: &tauri::WebviewWindow) {
    if main.is_minimized().unwrap_or(false) {
        let _ = main.unminimize();
    }

    // If persisted coordinates are fully outside the current monitor, re-center.
    if let (Ok(Some(monitor)), Ok(pos), Ok(size)) = (
        main.current_monitor(),
        main.outer_position(),
        main.outer_size(),
    ) {
        let mpos = monitor.position();
        let msize = monitor.size();

        let win_left = pos.x as i64;
        let win_top = pos.y as i64;
        let win_right = win_left + size.width as i64;
        let win_bottom = win_top + size.height as i64;

        let mon_left = mpos.x as i64;
        let mon_top = mpos.y as i64;
        let mon_right = mon_left + msize.width as i64;
        let mon_bottom = mon_top + msize.height as i64;

        let fully_offscreen = win_right <= mon_left
            || win_left >= mon_right
            || win_bottom <= mon_top
            || win_top >= mon_bottom;

        if fully_offscreen {
            let _ = main.center();
        }
    }

    let _ = main.show();
    let _ = main.set_focus();
}

/// Enforce a screen-relative minimum window size.
/// If the persisted (or default) size is smaller than 75% of the monitor,
/// resize to that target and re-center.  Floor: 1000 × 680 physical px.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn enforce_main_window_size(main: &tauri::WebviewWindow) {
    let monitor = main
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| main.primary_monitor().ok().flatten());

    let (target_w, target_h) = if let Some(mon) = monitor {
        let ms = mon.size();
        (
            ((ms.width  as f64 * 0.75) as u32).max(1000),
            ((ms.height as f64 * 0.75) as u32).max(680),
        )
    } else {
        (1440, 900)
    };

    let current = main.outer_size().unwrap_or(PhysicalSize::new(0, 0));
    if current.width < target_w || current.height < target_h {
        let _ = main.set_size(Size::Physical(PhysicalSize::new(target_w, target_h)));
        let _ = main.center();
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn persist_main_window_state(app: &tauri::AppHandle, window: &Window) {
    if let Ok(mut cfg) = crate::config::load_config(app) {
        if let Ok(pos) = window.outer_position() {
            cfg.main_window_x = Some(pos.x);
            cfg.main_window_y = Some(pos.y);
        }
        if let Ok(size) = window.outer_size() {
            cfg.main_window_width = Some(size.width);
            cfg.main_window_height = Some(size.height);
        }
        let _ = crate::config::save_config(app, &cfg);
    }
}

fn persist_assistant_visibility(app: &tauri::AppHandle, visible: bool) {
    if let Ok(mut cfg) = crate::config::load_config(app) {
        cfg.assistant_window_open = visible;
        let _ = crate::config::save_config(app, &cfg);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init());

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let builder = builder.plugin(tauri_plugin_window_state::Builder::default().build());

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        if let Some(window) = app.get_webview_window("main") {
            ensure_main_window_visible(&window);
        }
    }));

    #[cfg(any(target_os = "android", target_os = "ios"))]
    let builder = builder.plugin(tauri_plugin_barcode_scanner::init());

    builder
        .setup(move |app| {
            use tauri::Manager;
            let app_handle = app.handle().clone();

            // ── Structured logging (tracing) ──────────────────────────────────
            {
                use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
                let log_dir = app_handle.path().app_local_data_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                    .join("logs");
                let _ = std::fs::create_dir_all(&log_dir);
                let file_appender = tracing_appender::rolling::daily(&log_dir, "bonsai.log");
                let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
                let _ = LOG_GUARD.set(guard);
                let filter = tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
                let _ = tracing_subscriber::registry()
                    .with(tracing_subscriber::fmt::layer().json().with_writer(non_blocking).with_ansi(false))
                    .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
                    .with(filter)
                    .try_init();
                tracing::info!("Bonsai Workspace starting — logs: {}", log_dir.display());
            }

            // WAL — must be ready before any command runs
            let wal = Arc::new(
                tauri::async_runtime::block_on(wal::WAL::new(&app_handle))
                    .expect("WAL init failed"),
            );

            let chat_sessions = Arc::new(
                tauri::async_runtime::block_on(chat_sessions::ChatSessionStore::new(wal.pool()))
                    .expect("Chat sessions store init failed"),
            );

            // Check first-run bootstrap status
            let status = bootstrap::check_status(&app_handle);

            // Load config early so we can pass extra model dirs to the orchestrator.
            let early_cfg = config::load_config(&app_handle).unwrap_or_default();

            // Auto-register D:\Models\general on Windows if it exists and isn't already listed.
            {
                let general_dir = std::path::PathBuf::from(r"D:\Models\general");
                let general_str = general_dir.display().to_string();
                if general_dir.exists() && !early_cfg.extra_model_dirs.iter().any(|d| d == &general_str) {
                    let mut patched = early_cfg.clone();
                    patched.extra_model_dirs.push(general_str);
                    let _ = config::save_config(&app_handle, &patched);
                }
            }
            let early_cfg = config::load_config(&app_handle).unwrap_or_default();

            // Model orchestrator — starts event loop immediately;
            // slots go to Crashed if llama-server isn't present yet.
            let extra_model_dirs: Vec<std::path::PathBuf> = early_cfg.extra_model_dirs
                .iter()
                .map(std::path::PathBuf::from)
                .filter(|p| p.exists())
                .collect();
            let orchestrator = Arc::new(
                model_orchestrator::ModelOrchestrator::new(app_handle.clone(), extra_model_dirs),
            );

            // Whisper manager — best-effort spawn (no-op if binary absent)
            let whisper = Arc::new(sidecar_manager::WhisperManager::new(&app_handle));

            // Remote control subsystem — Phase 1 scaffold only.
            let remote_manager = Arc::new(remote::RemoteManager::new(&app_handle));

            let pty_sessions: Arc<Mutex<std::collections::HashMap<String, PtySession>>> =
                Arc::new(Mutex::new(std::collections::HashMap::new()));

            let agent_store = Arc::new(
                tauri::async_runtime::block_on(agent_store::AgentStore::new(wal.pool()))
                    .expect("Agent store init failed"),
            );

            let swarm_orch = Arc::new(swarm_orchestrator::SwarmOrchestrator::new(
                orchestrator.clone(),
            ));
            let cluster_orchestrator = Arc::new(Mutex::new(cluster_orchestrator::ClusterOrchestrator::new()));

            let bootstrap_cancel = Arc::new(AtomicBool::new(false));
            let chat_cancel = Arc::new(AtomicBool::new(false));
            let voice_cancel = Arc::new(AtomicBool::new(false));
            let agent_connect = Arc::new(StdMutex::new(agent_connect::AgentConnectHub::new()));

            // Generate an 8-char alphanumeric pairing token for Android / VSCode.
            use rand::distributions::Alphanumeric;
            use rand::Rng;
            let pair_token: String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(8)
                .map(char::from)
                .collect();

            let ws_router = Arc::new(ws_router::WsRouter::new());

            let policy_engine = Arc::new(assistant_policy::PolicyEngine::new());
            let confirmation_gate = Arc::new(assistant_policy::ConfirmationGate::new());
            let audit_log = Arc::new(assistant_audit_log::AuditLog::new(
                app_handle.path().app_data_dir().expect("app data dir"),
            ));
            let secrets_store = Arc::new(secrets_store::SecretsStore::new());
            let assistant_store_inst = Arc::new(
                tauri::async_runtime::block_on(
                    assistant_store::AssistantStore::new(wal.pool())
                ).expect("assistant store init failed"),
            );
            let assistant_cancel = Arc::new(AtomicBool::new(false));
            let asst_metrics = Arc::new(assistant_metrics::AssistantMetrics::new());
            let tts_manager = Arc::new(tts_manager::TtsManager::new(&app_handle));
            let user_skill_store = Arc::new(
                tauri::async_runtime::block_on(
                    user_skills::UserSkillStore::new(wal.pool())
                ).expect("user skill store init failed"),
            );

            // MCP bridge — load persisted configs and connect on startup
            let mcp_manager = Arc::new(mcp_bridge::McpManager::new());

            let model_data_store = Arc::new(
                tauri::async_runtime::block_on(
                    model_data_store::ModelDataStore::new(wal.pool())
                ).expect("model data store init failed"),
            );

            let task_queue = Arc::new(task_queue::TaskQueue::new(
                orchestrator.clone(),
                task_queue::QueueConfig::default(),
            ));

            let mut api_config = config::load_config(&app_handle).unwrap_or_default();

            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            restore_main_window_state(&app_handle, &api_config);
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            if let Some(main_win) = app_handle.get_webview_window("main") {
                enforce_main_window_size(&main_win);
            }

            // Respect explicit `BONSAI_API_PORT` environment override when present.
            // Otherwise, keep the persisted `api_port` (if non-zero). If the persisted
            // value is zero/unset, fall back to the default and persist it.
            if let Ok(val) = std::env::var("BONSAI_API_PORT") {
                if let Ok(p) = val.parse::<u16>() {
                    api_config.api_port = p;
                }
            } else if api_config.api_port == 0 {
                api_config.api_port = config::DEFAULT_API_PORT;
                let _ = config::save_config(&app_handle, &api_config);
            }

            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            if api_config.assistant_window_open {
                if let Some(assistant_window) = app.get_webview_window("assistant") {
                    let _ = assistant_window.show();
                }
            }

            let api_runtime = {
                let orch   = orchestrator.clone();
                let remote = remote_manager.clone();
                let ws     = ws_router.clone();
                let token  = pair_token.clone();
                let host   = api_config.api_host.clone();
                let port   = api_config.api_port;
                // Try the preferred port and a small range of fallback ports if binding fails.
                // This avoids hard failures when the preferred port is briefly unavailable
                // (e.g., stale listeners or other local tools). We attempt +1..+4 as fallbacks.
                match tauri::async_runtime::block_on(api_server::start_with_fallback(
                    orch,
                    remote,
                    ws,
                    token,
                    host,
                    port,
                    4u16,
                    app_handle.clone(),
                )) {
                    Ok(handle) => Some(handle),
                    Err(e) => {
                        tracing::error!("[api] failed to start API server: {e}");
                        None
                    }
                }
            };

            if let Some(ref handle) = api_runtime {
                if handle.host != api_config.api_host || handle.port != api_config.api_port {
                    api_config.api_host = handle.host.clone();
                    api_config.api_port = handle.port;
                    let _ = config::save_config(&app_handle, &api_config);
                }
            }

            // ── Buddy API server (port 11420) ─────────────────────────────────
            let buddy_preferred = api_config.buddy_api_port;
            let (buddy_handle, buddy_port) = {
                let orch  = orchestrator.clone();
                let store = assistant_store_inst.clone();
                let pe    = policy_engine.clone();
                let gate  = confirmation_gate.clone();
                let audit = audit_log.clone();
                let sec   = secrets_store.clone();
                let bh    = app_handle.clone();
                match tauri::async_runtime::block_on(buddy_api_server::start(
                    orch, store, pe, gate, audit, sec, bh, buddy_preferred,
                )) {
                    Ok(h) => {
                        let p = h.port;
                        (Some(h), p)
                    }
                    Err(e) => {
                        tracing::error!("[buddy-api] failed to start: {e}");
                        let _ = app_handle.emit("buddy-api-unavailable", e);
                        (None, 0u16)
                    }
                }
            };

            // ── Agent Host — built-in agent registry ──────────────────────────
            let agent_host = Arc::new(agent_host::AgentHost::new());
            {
                let host = agent_host.clone();
                tauri::async_runtime::spawn(async move {
                    host.register(Arc::new(agents::code_writer::CodeWriter)).await;
                    host.register(Arc::new(agents::code_reviewer::CodeReviewer)).await;
                    tracing::info!("[agent-host] Built-in agents registered (code-writer, code-reviewer)");
                });
            }

            app.manage(AppState {
                orchestrator:     orchestrator.clone(),
                whisper:          whisper.clone(),
                wal,
                chat_sessions:    chat_sessions.clone(),
                pty_sessions,
                bootstrap_cancel: bootstrap_cancel.clone(),
                chat_cancel:      chat_cancel.clone(),
                voice_cancel:     voice_cancel.clone(),
                agent_connect,
                ws_router:          ws_router.clone(),
                pair_token:         pair_token.clone(),
                agent_store,
                swarm_orchestrator: swarm_orch,
                swarm_cancels:      Arc::new(StdMutex::new(HashMap::new())),
                api_server:         Arc::new(Mutex::new(api_runtime)),
                cluster_orchestrator,
                policy_engine,
                confirmation_gate,
                audit_log,
                secrets_store,
                assistant_store:  assistant_store_inst,
                assistant_cancel,
                asst_metrics,
                tts_manager,
                user_skill_store,
                mcp_manager: mcp_manager.clone(),
                buddy_api_server: Arc::new(Mutex::new(buddy_handle)),
                buddy_api_port:   buddy_port,
                model_data_store: model_data_store.clone(),
                task_queue,
                agent_host,
            });
            app.manage(remote_manager.clone());
            app.manage(features::FeatureFlags::global());

            // ── Startup health gate ────────────────────────────────────────────
            // Check whether each major subsystem initialised. Emit a single event
            // shortly after the window opens so the frontend can surface problems
            // instead of presenting a silent blank state.
            {
                let bh   = app_handle.clone();
                let orch = orchestrator.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                    let model_ready = orch.active_slot_url().await.is_some();
                    if model_ready {
                        tracing::info!("startup: AI model slot is ready");
                    } else {
                        tracing::warn!("startup: no AI model slot is ready — prompting user");
                        let _ = bh.emit("startup-health", serde_json::json!({
                            "model_ready": false,
                            "message": "No AI model is loaded. Go to Settings → Models to download or configure one."
                        }));
                    }
                });
            }

            // ── Model data: sync registry → store on startup ──────────────
            {
                let mds  = model_data_store.clone();
                let orch = orchestrator.clone();
                let default_mode = early_cfg.default_inference_mode.clone();
                tauri::async_runtime::spawn(async move {
                    let models = orch.list_models().await;
                    match mds.sync_from_registry(&models, &default_mode).await {
                        Ok(n) if n > 0 => tracing::info!("[model-data] created {n} skeleton entries from registry"),
                        Ok(_)          => {},
                        Err(e)         => tracing::warn!("[model-data] registry sync failed: {e}"),
                    }

                    if let Ok(entries) = mds.list().await {
                        for d in entries {
                            if let crate::model_data::ModelSource::LocalGguf { registry_id: Some(id), .. } = d.source {
                                orch.set_inference_mode(id, d.inference_mode.clone());
                            }
                        }
                    }
                });
            }

            // ── User skills startup load ────────────────────────────────────
            {
                let user_skills = app.state::<AppState>().user_skill_store.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = crate::assistant_manager::reload_user_skills(&user_skills).await {
                        tracing::error!("[skills] failed to load user skills at startup: {e}");
                    }
                });
            }

            // ── MCP startup connect ────────────────────────────────────────────
            {
                let store   = app.state::<AppState>().assistant_store.clone();
                let mgr     = mcp_manager.clone();
                let app_for_cfg = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    match store.list_mcp_servers().await {
                        Ok(configs) => {
                            let allowed_commands = crate::config::load_config(&app_for_cfg)
                                .map(|c| c.mcp_allowed_commands)
                                .unwrap_or_default();
                            mgr.load_configs(configs).await;
                            let registry = crate::assistant_manager::assistant_registry();
                            let mut reg = registry.write().await;
                            let connected = mgr.connect_all_into_registry(&mut *reg, &allowed_commands).await;
                            if !connected.is_empty() {
                                tracing::info!("[mcp] connected: {}", connected.join(", "));
                            }
                        }
                        Err(e) => tracing::error!("[mcp] failed to load configs: {e}"),
                    }
                });
            }

            // ── Sidecar watchdog ───────────────────────────────────────────────
            // Pings whisper + llama-server every 10s; emits assistant-health.
            {
                let bh      = app_handle.clone();
                let state   = app.state::<AppState>();
                let whisper = state.whisper.clone();
                let orch    = state.orchestrator.clone();
                let metrics = state.asst_metrics.clone();
                tauri::async_runtime::spawn(async move {
                    use tauri::Emitter;
                    let http = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(3))
                        .build()
                        .unwrap_or_default();
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

                        // Ping whisper health endpoint
                        let whisper_url = format!("{}/health", whisper.url());
                        let whisper_ok = http.get(&whisper_url).send().await
                            .map(|r| r.status().is_success())
                            .unwrap_or(false);

                        // Check orchestrator for a ready slot
                        let orch_ok = orch.active_slot_url().await.is_some();

                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default().as_secs() as i64;

                        let health = assistant_metrics::AssistantHealth {
                            sidecars: vec![
                                assistant_metrics::SidecarHealth {
                                    name: "whisper".into(),
                                    healthy: whisper_ok,
                                    last_checked_ts: ts,
                                    consecutive_failures: if whisper_ok { 0 } else { 1 },
                                },
                                assistant_metrics::SidecarHealth {
                                    name: "llama-server".into(),
                                    healthy: orch_ok,
                                    last_checked_ts: ts,
                                    consecutive_failures: 0,
                                },
                            ],
                            db_ok: true,
                            last_error: None,
                            checked_at: ts,
                        };

                        if !whisper_ok || !orch_ok {
                            metrics.record_error(
                                "watchdog",
                                &format!("whisper={whisper_ok} llama={orch_ok}"),
                            );
                        }

                        let _ = bh.emit_to(
                            tauri::EventTarget::webview_window("assistant"),
                            "assistant-health",
                            &health,
                        );
                    }
                });
            }

            // ── BonsaiBot status polling loop ──────────────────────────────────
            // Polls the bot admin API every 10 s and emits `bot-status-changed`
            // to the main window so ResourcesPanel can update without manual refresh.
            {
                let bh2 = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    use tauri::Emitter;
                    let http = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(3))
                        .build()
                        .unwrap_or_default();
                    let mut last_online: Option<bool> = None;
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

                        let port = crate::commands::read_persisted_bot_port()
                            .ok()
                            .flatten();
                        let port = match port { Some(p) => p, None => continue };

                        // Read token from keyring (best-effort; empty string if unavailable)
                        let token = keyring::Entry::new("bonsai-bot", "bot_admin_token")
                            .ok()
                            .and_then(|e| e.get_password().ok())
                            .unwrap_or_default();

                        let url = format!("http://127.0.0.1:{port}/status");
                        match http.get(&url)
                            .header("authorization", format!("Bearer {token}"))
                            .send().await
                        {
                            Ok(resp) if resp.status().is_success() => {
                                if let Ok(body) = resp.json::<serde_json::Value>().await {
                                    if last_online != Some(true) {
                                        tracing::info!("[bot-watchdog] bot came online");
                                        last_online = Some(true);
                                    }
                                    let _ = bh2.emit("bot-status-changed", serde_json::json!({
                                        "online":  true,
                                        "status":  body,
                                    }));
                                }
                            }
                            _ => {
                                if last_online != Some(false) {
                                    tracing::info!("[bot-watchdog] bot went offline");
                                    last_online = Some(false);
                                }
                                let _ = bh2.emit("bot-status-changed", serde_json::json!({
                                    "online": false,
                                }));
                            }
                        }
                    }
                });
            }

            // Register mDNS service so Android can discover this desktop on the LAN.
            {
                let port = api_config.api_port;
                tauri::async_runtime::spawn_blocking(move || {
                    use mdns_sd::{ServiceDaemon, ServiceInfo};
                    match ServiceDaemon::new() {
                        Ok(mdns) => {
                            let hostname = gethostname::gethostname()
                                .to_string_lossy()
                                .replace(' ', "-");
                            let fqdn = format!("{hostname}.local.");
                            match ServiceInfo::new(
                                "_bonsai._tcp.local.",
                                "Bonsai Workspace",
                                &fqdn,
                                "",
                                port,
                                None,
                            ) {
                                Ok(svc) => { let _ = mdns.register(svc); }
                                Err(e) => tracing::warn!("[mdns] service info error: {e}"),
                            }
                        }
                        Err(e) => tracing::warn!("[mdns] daemon error: {e}"),
                    }
                });
            }

            if !status.all_ready() {
                // Tell the frontend to show the bootstrap/setup screen
                let _ = app_handle.emit("bootstrap-needed", &status);

                // Run bootstrap in the background; on success, refresh the
                // orchestrator so it picks up the freshly-downloaded model.
                let bh     = app_handle.clone();
                let orch   = orchestrator.clone();
                let cancel = bootstrap_cancel.clone();
                tauri::async_runtime::spawn(async move {
                    match bootstrap::run(bh.clone(), cancel).await {
                        Ok(()) => {
                            orch.refresh_registry();
                        }
                        Err(e) => {
                            tracing::error!("[bootstrap] failed: {e}");
                            let _ = bh.emit("bootstrap-error", e.to_string());
                        }
                    }
                });
            }

            // Background RAG index: walk workspace root for searchable documents.
            {
                let ws_root = app_handle.path().app_local_data_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                    .display().to_string();
                tauri::async_runtime::spawn_blocking(move || {
                    crate::rag_store::index_directory(&ws_root, 2000);
                });
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                if window.label() == "main" {
                    if matches!(event, tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)) {
                        persist_main_window_state(&window.app_handle(), window);
                    }
                }
            }

            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "assistant" {
                    persist_assistant_visibility(&window.app_handle(), false);
                }

                #[cfg(not(any(target_os = "android", target_os = "ios")))]
                if window.label() == "main" {
                    let app = window.app_handle();
                    persist_main_window_state(&app, window);

                    let assistant_visible = app
                        .get_webview_window("assistant")
                        .and_then(|w| w.is_visible().ok())
                        .unwrap_or(false);
                    persist_assistant_visibility(&app, assistant_visible);

                    if app.get_webview_window("assistant").is_some() {
                        let _ = window.hide();
                        api.prevent_close();
                        return;
                    }
                    // No assistant window — do auto-backup then let close proceed
                    if let Some(state) = app.try_state::<AppState>() {
                        let store   = state.assistant_store.clone();
                        let app2    = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = assistant_backup::export_backup(
                                &app2, &store, true, false, false, None,
                            ).await;
                        });
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            // ── File system ───────────────────────────────────────────────────
            commands::read_file,
            commands::write_file,
            commands::load_canvas_layout,
            commands::save_canvas_layout,
            commands::create_directory,
            commands::delete_file,
            commands::list_project_files,
            // ── Git ───────────────────────────────────────────────────────────
            commands::get_git_status,
            commands::get_git_branch,
            // ── AI / Chat ─────────────────────────────────────────────────────
            commands::submit_chat,
            commands::resume_tool_call,
            commands::stop_chat_generation,
            commands::list_available_chat_tools,
            commands::generate_inline_completion,
            commands::execute_tool_call,
            commands::list_chat_sessions,
            commands::list_chat_sessions_detailed,
            commands::save_chat_session,
            commands::load_chat_session,
            commands::delete_chat_session,
            commands::rename_chat_session,
            commands::duplicate_chat_session,
            commands::update_chat_session_meta,
            commands::list_chat_session_groups,
            commands::create_chat_session_group,
            commands::update_chat_session_group_meta,
            commands::link_chat_to_session_group,
            commands::unlink_chat_from_session_group,
            commands::list_group_chats,
            commands::voice_transcribe,
            commands::stop_voice_capture,
            commands::ai_scaffold_project,
            commands::ai_code_review,
            commands::agent_connect_start_session,
            commands::agent_connect_set_active_session,
            commands::agent_connect_get_active_session,
            commands::agent_connect_list_sessions,
            commands::agent_connect_get_timeline,
            commands::agent_connect_end_session,
            // ── Models ────────────────────────────────────────────────────────
            commands::list_available_models,
            commands::list_models_registry,
            commands::switch_model,
            commands::load_model,
            commands::unload_slot,
            commands::get_orchestrator_status,
            commands::get_hardware_info,
            commands::get_api_port,
            commands::get_buddy_api_port,
            commands::get_api_config,
            commands::set_api_config,
            commands::get_current_session_state,
            commands::set_current_session_state,
            commands::start_remote_session,
            commands::stop_remote_session,
            commands::send_remote_input,
            commands::prompt_gguf_import,
            // ── Bootstrap ─────────────────────────────────────────────────────
            commands::check_bootstrap_status,
            commands::run_bootstrap,
            commands::cancel_bootstrap,
            // ── Downloads ────────────────────────────────────────────────────
            commands::download_gguf_model,
            commands::download_whisper_model,
            // ── Terminal / PTY ────────────────────────────────────────────────
            commands::run_terminal_command,
            commands::spawn_pty_terminal,
            commands::send_to_pty,
            commands::send_to_pty_session,
            commands::resize_pty,
            commands::resize_pty_session,
            commands::close_pty_session,
            commands::open_workspace,
            // ── Diff ─────────────────────────────────────────────────────────
            commands::accept_diff_hunk,
            commands::reject_diff_hunk,
            commands::create_unified_diff,
            commands::create_project_from_template,
            // ── Connection / pairing ──────────────────────────────────────────
            commands::get_pair_token,
            commands::get_local_ip,
            commands::generate_pair_qr,
            commands::scan_qr,
            commands::save_desktop_connection,
            commands::load_desktop_connection,
            commands::android_usb_list_devices,
            commands::android_usb_get_adb_info,
            commands::android_mobile_view_status,
            commands::android_mobile_cancel_pending_operations,
            commands::android_mobile_view_start,
            commands::android_mobile_view_stop,
            commands::android_mobile_take_screenshot,
            commands::android_mobile_start_recording,
            commands::android_mobile_stop_recording,
            commands::android_mobile_launch_camera,
            commands::android_mobile_send_key,
            commands::android_mobile_send_text,
            commands::android_mobile_tap,
            commands::android_mobile_swipe,
            commands::android_mobile_get_display_info,
            commands::android_mobile_set_orientation,
            commands::android_mobile_launch_bonsai,
            commands::android_mobile_prepare_uniform_runtime,
            commands::android_usb_shell,
            commands::android_usb_install_apk,
            commands::android_usb_launch_app,
            commands::android_usb_reverse,
            commands::android_usb_reverse_clear,
            commands::android_usb_enable_wifi_debug,
            commands::android_usb_connect_wifi,
            commands::android_usb_disconnect_wifi,
            commands::android_usb_run_regression,
            commands::android_usb_get_device_readiness,
            commands::android_usb_resolve_apk,
            commands::android_usb_install_and_launch,
            commands::android_usb_bootstrap_connection,
            commands::record_mobile_pairing_evidence,
            commands::get_mobile_pairing_evidence,
            commands::browse_bonsai_services,
            commands::ws_broadcast,
            commands::ws_client_count,
            // ── Multi-agent swarm ─────────────────────────────────────────────
            commands::list_personas,
            commands::upsert_persona,
            commands::delete_persona,
            commands::list_agent_configs,
            commands::upsert_agent_config,
            commands::delete_agent_config,
            commands::estimate_swarm_resources,
            commands::submit_swarm_chat,
            commands::cancel_swarm,
            commands::cancel_agent,
            commands::get_swarm_metrics,
            // ── Bonsai Assistant ──────────────────────────────────────────────
            assistant_commands::list_assistant_profiles,
            assistant_commands::get_active_assistant_profile,
            assistant_commands::upsert_assistant_profile,
            assistant_commands::delete_assistant_profile,
            assistant_commands::set_active_assistant_profile,
            assistant_commands::list_avatar_assets,
            assistant_commands::upsert_avatar_asset,
            assistant_commands::delete_avatar_asset,
            assistant_commands::list_assistant_sessions,
            assistant_commands::create_assistant_session,
            assistant_commands::load_assistant_session,
            assistant_commands::delete_assistant_session,
            assistant_commands::toggle_assistant_window,
            assistant_commands::toggle_android_usb_lab_window,
            assistant_commands::set_assistant_always_on_top,
            assistant_commands::set_smtp_credentials,
            assistant_commands::has_smtp_credentials,
            assistant_commands::clear_smtp_credentials,
            assistant_commands::submit_assistant_chat,
            assistant_commands::stop_assistant_chat,
            assistant_commands::confirm_tool_action,
            assistant_commands::cancel_tool_action,
            assistant_commands::get_assistant_audit_log,
            assistant_commands::get_assistant_metrics,
            assistant_commands::get_assistant_health,
            assistant_commands::export_assistant_backup,
            assistant_commands::import_assistant_backup,
            assistant_commands::list_assistant_backups,
            assistant_commands::verify_backup_integrity,
            assistant_commands::delete_assistant_backup_entry,
            assistant_commands::auto_title_session,
            assistant_commands::speak_text,
            assistant_commands::stop_tts,
            assistant_commands::set_tts_voice,
            assistant_commands::set_tts_speed,
            assistant_commands::is_tts_available,
            assistant_commands::validate_avatar_svg,
            assistant_commands::list_user_skills,
            assistant_commands::upsert_user_skill,
            assistant_commands::delete_user_skill,
            assistant_commands::test_user_skill,
            // ── MCP bridge ────────────────────────────────────────────────────
            assistant_commands::list_mcp_servers,
            assistant_commands::upsert_mcp_server,
            assistant_commands::delete_mcp_server,
            assistant_commands::reconnect_mcp_servers,
            // ── Cluster orchestrator ──────────────────────────────────────────
            commands::cluster_list_nodes,
            commands::cluster_upsert_node,
            commands::cluster_remove_node,
            commands::cluster_update_node_metrics,
            commands::cluster_set_policy,
            commands::cluster_get_policy,
            commands::cluster_plan_workload,
            // ── Messaging bot ─────────────────────────────────────────────────
            commands::get_bot_server_status,
            commands::get_bot_metrics,
            commands::save_discord_bot_config,
            commands::save_telegram_bot_config,
            commands::save_matrix_bot_config,
            commands::save_email_bot_config,
            commands::test_bot_platform,
            commands::get_matrix_key_backup_passphrase,
            // ── Model Data ────────────────────────────────────────────────────
            commands::list_model_data,
            commands::get_model_data,
            commands::save_model_data,
            commands::delete_model_data,
            commands::search_model_data,
            commands::rank_models_for_skill,
            commands::generate_model_data,
            commands::sync_registry_to_model_data,
            commands::get_default_inference_mode,
            commands::set_default_inference_mode,
            commands::get_inference_mode,
            commands::set_inference_mode,
            commands::apply_inference_mode_to_all,
            commands::get_task_queue_status,
            // ── Model directories ─────────────────────────────────────────────
            commands::list_model_directories,
            commands::add_model_directory,
            commands::remove_model_directory,
            // ── Feature flags ─────────────────────────────────────────────────
            features::get_feature_flags,
            features::set_feature_flags,
            // ── Agent Host ────────────────────────────────────────────────────
            commands::list_agents,
            commands::send_agent_message,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
