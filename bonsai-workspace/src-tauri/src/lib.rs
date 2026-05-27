#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]

use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

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
mod capability_commands;
mod thoughts;
mod thoughts_commands;
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
mod management_api;
pub mod bonsai_core;
pub mod data_curator;
pub mod telemetry;
mod trainer;
mod hybrid_engine;
mod launcher;
mod gpu_layer;
mod gpu_telemetry;
mod gpu_model_loader;
mod dual_inference;
mod training_loop;
mod rich_markdown;
mod sandbox_executor;
mod image_generation;
mod tts_engine;
mod sidecar_supervisor;
mod plugin_loader;
mod plugin_manifest;
mod plugin_host;
mod a2a_server;
mod p2p {
    pub mod sharing;
}
mod collab {
    pub mod crdt;
}
mod tool_registry;
mod self_play;
mod critic;
mod adapter_manager;
mod tool_watcher;
mod mcp_server;
mod tool_compose;
mod cross_training;
mod swarm_orchestrator;
mod shared_arena;
mod micro_bonsai;
mod swarm_config;
mod gpu_controller;
mod multimodal;
mod vision_training;
mod task_queue;
mod tools;
mod music_engine;
mod user_skills;
mod wal;
mod ws_router;
mod skill_registry;
mod expanded_tools;
mod skill_compiler_commands;

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

// Workstream modules (scaffolds)
mod auth_commands;
mod marketplace_commands;
mod meeting_agent;
mod continuous_training;
mod unified_training_collector;
mod evaluation_harness;
mod promotion_gate;
mod forgetting_prevention;
mod eternal_training_loop;
mod training_commands;
mod nn_commands;
mod orchestrator;
mod sylva;
mod federated_trainer;
mod games;
mod belief_reviser;
mod metacognitive_monitor;
mod reasoning_engine;
mod knowledge_tools;
mod omnipresent_capture;
mod predictive_engine;
mod omnfs;
mod omni_desktop;
mod omni_shell;
mod process_manager;
mod omni_session;
mod device_manager;
mod omni_boot;

// Workstream types
use crate::auth_commands::AuthState;
use crate::marketplace_commands::MarketState;
use crate::meeting_agent::MeetingAgent;
use crate::continuous_training::ContinuousTrainer;

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
    /// Workstream: multi-user auth & encrypted workspace state.
    pub auth_state:       Arc<AuthState>,
    /// Workstream: marketplace state.
    pub market_state:     Arc<MarketState>,
    /// Workstream: meeting/conference agent.
    pub meeting_agent:    Arc<MeetingAgent>,
    /// Workstream: continuous fine-tuning trainer.
    pub continuous_trainer: Arc<ContinuousTrainer>,
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
    /// BonsAI-Core orchestrator — plan, execute, curate.
    pub bonsai_core:      Arc<bonsai_core::BonsaiCore>,
    /// Telemetry store — training runs + inference metrics.
    pub telemetry:        Arc<telemetry::TelemetryStore>,
    /// Native llama.cpp Vulkan engine (AMD 7900 XTX GPU inference).
    pub hybrid_engine:    Arc<hybrid_engine::HybridEngineState>,
    /// GPU layer health tracker + VRAM estimator.
    pub gpu:              Arc<gpu_layer::GpuLayer>,
    /// Long-lived dual model session manager for continuous training loop.
    pub dual_session:     Arc<dual_inference::SessionManager>,
    /// Controlled continuous training loop orchestrator.
    pub training_loop:    Arc<training_loop::TrainingLoopState>,
    /// Self-play training loop (generate → critique → correct → curate).
    pub self_play:        Arc<self_play::SelfPlayState>,
    /// Unified training container (collector, loop engine, adapter registry).
    pub training:         Arc<training_commands::TrainingState>,
    /// Secure WASM/Python plugin host with capability enforcement.
    pub plugin_host:      Arc<plugin_host::PluginHost>,
    /// Pluggable tool registry (execute_code, system_info, …).
    pub tool_registry:    Arc<tool_registry::ToolRegistryState>,
    /// Universal Capability Registry (UCR) — aggregated capability manifest
    pub capability_registry: Arc<bonsai_capability_registry::UniversalCapabilityRegistry>,
    /// Thoughts DB store — persistent model thinking capture
    pub thoughts_db: Arc<thoughts::ThoughtsStore>,
    /// Cross-training event sender — feed chat/plugin/tool events for passive data collection.
    pub cross_training:   cross_training::CrossTrainingSender,
    /// MCP server port (0 if startup failed). Exposes all tools to Claude Desktop, Cursor, etc.
    pub mcp_port:         u16,
    /// Filesystem watcher — invalidates tool cache on workspace file changes.
    pub tool_watcher:     Arc<Mutex<Option<tool_watcher::ToolWatcher>>>,
    /// Zero-copy mmap-backed cross-model memory arena.
    pub shared_arena:     Arc<shared_arena::SharedMemoryArena>,
    /// Micro BonsAI intelligent model monitor and selector.
    pub micro_bonsai:     Arc<micro_bonsai::MicroBonsai>,
    /// Custom swarm configuration store (SQLite-backed).
    pub swarm_config_store: Arc<swarm_config::SwarmConfigStore>,
    /// Unified GPU controller — layer allocation, health, invisible crash recovery.
    pub gpu_controller:    Arc<gpu_controller::GpuController>,
    /// Skills.sh-backed skill registry with custom-rebuild, security scanning, prompt injection.
    pub skill_registry:    Arc<skill_registry::SkillRegistryState>,
    /// Thinking settings — per-role visibility toggles, max tokens.
    pub thinking_settings: Arc<tokio::sync::RwLock<serde_json::Value>>,
    /// CAS (Content-Addressed Store) — Blake3-keyed deduplicating blob store.
    pub cas_store: Arc<bonsai_cas::CasStore>,
    /// Sylva scripting runtime — hot-reloadable Lua scripts as UCR tools.
    pub sylva: crate::sylva::SylvaState,
    /// Federated training coordinator — CRDT-backed multi-peer state.
    pub federated_trainer: Arc<crate::federated_trainer::FederatedTrainer>,
    /// Actor system — supervisor trees for swarm workers and background agents.
    pub actor_system: Arc<bonsai_actors::ActorSystem>,
    /// Chess and Go game session store.
    pub game_sessions: Arc<crate::games::GameSessionStore>,
    /// Knowledge graph — entities, relations, beliefs.
    pub knowledge: Arc<bonsai_knowledge::KnowledgeGraph>,
    /// Multi-strategy reasoning engine.
    pub reasoning: Arc<crate::reasoning_engine::ReasoningEngine>,
    /// Bayesian belief reviser with contradiction resolution.
    pub belief_reviser: Arc<tokio::sync::RwLock<crate::belief_reviser::BeliefReviser>>,
    /// Metacognitive monitor — calibration and strategy tracking.
    pub metacognitive: Arc<tokio::sync::RwLock<crate::metacognitive_monitor::MetacognitiveMonitor>>,
    /// Omnipresent event capture — records every user action.
    pub omnipresent: Arc<crate::omnipresent_capture::OmnipresentCapture>,
    /// Predictive engine — Markov + temporal models, automation rules.
    pub predictive_engine: Arc<crate::predictive_engine::PredictiveEngine>,
    /// OmnFS — CAS-backed virtual file system with versioning and snapshots.
    pub omnfs: Arc<crate::omnfs::OmnFS>,
    /// OmniDesktop — GPU-accelerated compositor, window manager, panel engine.
    pub omni_desktop: Arc<crate::omni_desktop::OmniDesktop>,
    /// OmniShell — AI-native terminal with prediction, NL translation, auto-fix.
    pub omni_shell: Arc<crate::omni_shell::OmniShellState>,
    /// Process Manager — TrustGuard-enforced process lifecycle, 4-tier sandboxing.
    pub process_manager: Arc<crate::process_manager::ProcessManager>,
    /// OmniSession — unified user environment: login, snapshot, session summary.
    pub omni_session: Arc<crate::omni_session::OmniSession>,
    /// Device Manager — peripheral enumeration, display config, hotplug.
    pub device_manager: Arc<crate::device_manager::DeviceManager>,
    /// OmniBoot — self-verifying boot chain with CAS manifest and Axiom proofs.
    pub omni_boot: Arc<crate::omni_boot::OmniBoot>,
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

/// Kill any llama-server or piper sidecar processes left over from a
/// previous session that crashed or was force-killed. This prevents
/// port accumulation and "connection refused" errors on restart.
fn sweep_stale_sidecars() {
    use sysinfo::{ProcessRefreshKind, RefreshKind, System};
    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::new()),
    );
    sys.refresh_processes();
    let sidecar_names = ["llama-server", "piper", "stable-diffusion"];
    let mut killed = 0u32;
    for (pid, proc) in sys.processes() {
        let name = proc.name().to_string();
        if sidecar_names.iter().any(|s| name.contains(s)) {
            if proc.kill() {
                killed += 1;
                tracing::info!(pid=%pid, name=%name, "[startup] swept stale sidecar");
            }
        }
    }
    if killed > 0 {
        // Give OS a moment to release ports before we try to bind.
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Write a crash dump on panic so users can report issues.
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("{info}");
        tracing::error!("PANIC: {msg}");
        let dump_path = std::env::temp_dir().join("bonsai_crash.txt");
        let _ = std::fs::write(&dump_path, &msg);
        eprintln!("Bonsai crashed. Dump written to: {}", dump_path.display());
    }));

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

            // Kill any stale llama-server processes from previous sessions.
            // These accumulate when the app crashes or is force-killed without
            // triggering Drop on SharedServer.
            sweep_stale_sidecars();

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

            // Initialize workstream scaffolds (A-D)
            let auth_state    = Arc::new(AuthState::new());
            let market_state  = Arc::new(MarketState::new());

            let (meeting_agent_inst, meeting_rx) = MeetingAgent::new();
            let meeting_agent = Arc::new(meeting_agent_inst);

            let continuous_trainer = Arc::new(ContinuousTrainer::new());

            // Forward meeting agent progress events to the frontend
            {
                let bh = app_handle.clone();
                let mut mrx = meeting_rx;
                tauri::async_runtime::spawn(async move {
                    while let Ok(progress) = mrx.recv().await {
                        let _ = bh.emit("meeting-progress", progress);
                    }
                });
            }

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

            tracing::info!("[api] pair_token={pair_token}");
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
            // Persist the pair token so bonsai-bot and local clients can read it
            // from bonsai-config.json without needing the UI.
            api_config.pair_token = pair_token.clone();
            let _ = config::save_config(&app_handle, &api_config);

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

            // ── BonsAI-Core — orchestrator + data curator ─────────────────────
            let bonsai_inference_url = format!(
                "http://127.0.0.1:{}/v1/chat/completions",
                api_config.api_port
            );
            let prompt_template = concat!(
                "You are BonsAI-Core. Tools: list_files, read_file, write_file, grep_files, run_command, search_files.\n",
                "Output JSON only: {{\"intent\":\"...\",\"reasoning\":\"...\",\"plan\":[{{\"tool\":\"...\",\"args\":{{}}}}],",
                "\"final_response\":null,\"confidence\":0.9}}\n",
                "Example: {{\"intent\":\"list workspace files\",\"reasoning\":\"user wants directory listing\",",
                "\"plan\":[{{\"tool\":\"list_files\",\"args\":{{\"path\":\".\",\"recursive\":false}}}}],",
                "\"final_response\":null,\"confidence\":0.95}}\n",
                "User request: {request}\nMemory: {memory}\nJSON:"
            ).to_string();
            let bonsai_home = dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".bonsai");

            // ── Telemetry store ────────────────────────────────────────────────
            let telemetry_store = Arc::new(
                tauri::async_runtime::block_on(
                    telemetry::TelemetryStore::new(
                        bonsai_home.join("telemetry.db").to_str().unwrap_or("telemetry.db")
                    )
                ).unwrap_or_else(|e| {
                    tracing::error!("[telemetry] failed to open DB: {e}");
                    panic!("telemetry DB required");
                })
            );
            let memory_path = bonsai_home.join("core_memory.jsonl");
            let curator_path = bonsai_home.join("curated_examples.jsonl");
            let workspace_root = app_handle
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let bonsai_memory = bonsai_core::CoreMemory::new(Some(memory_path));
            let bonsai_curator = data_curator::DataCurator::new(curator_path, prompt_template.clone());
            let shared_bonsai_core = Arc::new(bonsai_core::BonsaiCore::new(
                None,
                bonsai_inference_url,
                bonsai_memory,
                bonsai_curator,
                prompt_template,
                workspace_root,
                false,
            ));

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

            let swarm_cancels = Arc::new(StdMutex::new(HashMap::new()));

            // Build the shared SessionManager before api_runtime and AppState so both reuse it.
            let shared_dual_session = Arc::new(dual_inference::SessionManager::new());

            // Build new subsystems early so MgmtState and AppState share the same Arcs.
            let early_self_play = Arc::new(self_play::SelfPlayState::new(
                orchestrator.clone(),
                self_play::SelfPlayConfig::default(),
            ));
            let early_plugin_host = Arc::new(plugin_host::PluginHost::new());
            {
                let ph = early_plugin_host.clone();
                tauri::async_runtime::spawn(async move { ph.load_all().await; });
            }
            let early_tool_registry = tauri::async_runtime::block_on(
                tool_registry::ToolRegistryState::new_with_defaults()
            );

            // Build game sessions early so MgmtState and AppState share the same Arc.
            let early_game_sessions = crate::games::GameSessionStore::new();

            // Build knowledge graph + reasoning engine — shared across MgmtState and AppState.
            let early_knowledge = Arc::new(bonsai_knowledge::KnowledgeGraph::new());
            let early_reasoning = Arc::new(crate::reasoning_engine::ReasoningEngine::new(early_knowledge.clone()));
            let early_belief_reviser: Arc<tokio::sync::RwLock<crate::belief_reviser::BeliefReviser>> =
                Arc::new(tokio::sync::RwLock::new(crate::belief_reviser::BeliefReviser::new()));
            let early_metacognitive: Arc<tokio::sync::RwLock<crate::metacognitive_monitor::MetacognitiveMonitor>> =
                Arc::new(tokio::sync::RwLock::new(crate::metacognitive_monitor::MetacognitiveMonitor::new()));

            let api_runtime = {
                let orch   = orchestrator.clone();
                let remote = remote_manager.clone();
                let ws     = ws_router.clone();
                let token  = pair_token.clone();
                let host   = api_config.api_host.clone();
                let port   = api_config.api_port;
                let mgmt = management_api::MgmtState {
                    orchestrator:  orchestrator.clone(),
                    agent_host:    agent_host.clone(),
                    agent_store:   agent_store.clone(),
                    task_queue:    task_queue.clone(),
                    swarm_cancels: swarm_cancels.clone(),
                    app_handle:    app_handle.clone(),
                    pair_token:    token.clone(),
                    bonsai_core:   shared_bonsai_core.clone(),
                    telemetry:     telemetry_store.clone(),
                    dual_session:  shared_dual_session.clone(),
                    training_loop: Arc::new(training_loop::TrainingLoopState::new(
                        orchestrator.clone(),
                        telemetry_store.clone(),
                    )),
                    self_play:     early_self_play.clone(),
                    plugin_host:   early_plugin_host.clone(),
                    tool_registry: early_tool_registry.clone(),
                    game_sessions: early_game_sessions.clone(),
                    knowledge:     early_knowledge.clone(),
                    reasoning:     early_reasoning.clone(),
                    belief_reviser: early_belief_reviser.clone(),
                    metacognitive:  early_metacognitive.clone(),
                };
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

            // Use the early-built instances (shared with MgmtState).
            let self_play_state = early_self_play;
            let plugin_host     = early_plugin_host;
            let tool_registry   = early_tool_registry;

            // Register knowledge tools into the tool registry.
            {
                let tr = tool_registry.clone();
                let kg = early_knowledge.clone();
                let re = early_reasoning.clone();
                let br = early_belief_reviser.clone();
                tauri::async_runtime::spawn(async move {
                    knowledge_tools::register_knowledge_tools(&tr, re, kg, br).await;
                });
            }

            let (cross_pipeline, cross_tx) = cross_training::CrossTrainingPipeline::new(
                cross_training::CrossTrainingConfig::default(),
            );
            tauri::async_runtime::spawn(cross_pipeline.start());
            let cross_training_sender = cross_training::CrossTrainingSender(cross_tx);

            // ── Launch supervisor ─────────────────────────────────────────────
            // Runs in the background after servers bind; emits bonsai:launch-progress
            // events so the frontend never calls endpoints before they're ready.
            {
                let api_port   = api_config.api_port;
                let buddy_port = api_config.buddy_api_port;
                let ah         = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    use std::sync::Arc;
                    let specs = launcher::bonsai_specs::bonsai_components(api_port, buddy_port);
                    let sup   = Arc::new(launcher::LaunchSupervisor::new(specs));
                    match sup.clone().probe_all(Some(ah.clone())).await {
                        Ok(()) => {
                            tracing::info!("[launcher] all services ready");
                            let _ = ah.emit("bonsai:services-ready", ());
                            // Start background health monitor (30-second interval)
                            let mon_sup = sup.clone();
                            let mon_ah  = ah.clone();
                            tauri::async_runtime::spawn(async move {
                                mon_sup.monitor(mon_ah, std::time::Duration::from_secs(30)).await;
                            });
                        }
                        Err(e) => {
                            tracing::error!(error=%e, "[launcher] startup probe failed");
                            let _ = ah.emit("bonsai:services-failed", e);
                        }
                    }
                });
            }

            // ── MCP server (port 11421) ────────────────────────────────────────
            // Exposes all assistant tools to any MCP-compatible client (Claude Desktop,
            // Cursor, VS Code Continue) — Bonsai becomes the universal local tool backend.
            let mcp_port = {
                use tokio::sync::RwLock as TokioRwLock;
                let registry = Arc::new(TokioRwLock::new(crate::tool_core::ToolRegistry::new()));
                let memory_path = Some(
                    dirs::home_dir().unwrap_or_default().join(".bonsai/core_memory.jsonl")
                );
                match tauri::async_runtime::block_on(mcp_server::start(
                    registry,
                    None, // workspace root injected per-call from tool context
                    memory_path,
                    pair_token.clone(),
                    11421,
                )) {
                    Ok(h) => {
                        tracing::info!(port=h.port, "[mcp] MCP server ready");
                        h.port
                    }
                    Err(e) => {
                        tracing::warn!("[mcp] Failed to start: {e}");
                        0u16
                    }
                }
            };

            // ── Tool cache filesystem watcher ──────────────────────────────────
            // Invalidates cached tool results (read_file, grep_files, etc.) when
            // workspace files change — makes the tool cache safe AND fast.
            let tool_watcher_state: Arc<Mutex<Option<tool_watcher::ToolWatcher>>> =
                Arc::new(Mutex::new(None));

            // ── Zero-copy cross-model memory arena ────────────────────────────
            let arena_path = dirs::home_dir()
                .unwrap_or_default()
                .join(".bonsai/shared_arena.bin");
            let shared_arena = shared_arena::SharedMemoryArena::open(&arena_path, None)
                .unwrap_or_else(|e| {
                    tracing::warn!("[arena] Failed to open shared arena: {e}");
                    shared_arena::SharedMemoryArena::open(
                        std::env::temp_dir().join("bonsai_arena.bin"), None,
                    ).expect("Fallback arena creation failed")
                });

            // ── Micro BonsAI model monitor/selector ───────────────────────────
            let micro_bonsai = micro_bonsai::MicroBonsai::new();

            // ── Custom swarm configuration store ──────────────────────────────
            let swarm_config_store = tauri::async_runtime::block_on(
                swarm_config::SwarmConfigStore::new(wal.pool()),
            ).unwrap_or_else(|e| {
                panic!("[swarm_config] Failed to init store: {e}");
            });

            // ── Skill registry (skills.sh + local + Bonsai-native) ────────────
            let skill_registry = Arc::new(skill_registry::SkillRegistryState::new());

            // ── Unified GPU controller ─────────────────────────────────────────
            let gpu_ctrl_inst = Arc::new(gpu_layer::GpuLayer::new(&gpu_layer::GpuLayer::detect()));
            let gpu_controller = gpu_controller::GpuController::new(
                gpu_ctrl_inst.clone(),
                Arc::clone(&shared_arena),
                Arc::clone(&micro_bonsai),
            );
            // Run non-blocking startup health check.
            {
                let ctrl = Arc::clone(&gpu_controller);
                tauri::async_runtime::spawn(async move {
                    let report = gpu_controller::run_startup_health_check(&ctrl.gpu).await;
                    tracing::info!(
                        backend  = %report.backend,
                        healthy  = report.healthy,
                        vram_mb  = report.vram_free_mb,
                        "[gpu_ctrl] Startup health check"
                    );
                });
            }

            // Start VRAM TTL monitor (evict idle models) and profiling endpoint.
            {
                let gc = Arc::clone(&gpu_controller);
                tauri::async_runtime::spawn(async move {
                    // TTL: 5 minutes idle, check every 60 seconds
                    gc.run_ttl_monitor(300, 60).await;
                });
            }

            // Start GPU health probe (emits "gpu-unhealthy" event to frontend).
            {
                let gc = Arc::clone(&gpu_controller);
                gc.start_health_monitor(app_handle.clone(), 30);
            }

            // Start A2A server for agent interoperability (best-effort)
            {
                let ah = agent_host.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = tokio::spawn(async move { crate::a2a_server::start_a2a_server(11370, ah).await }).await {
                        tracing::error!("Failed to start A2A server: {:?}", e);
                    }
                });
            }

            // Start P2P announce (best-effort)
            {
                let _ = tauri::async_runtime::spawn(async move {
                    let _ = crate::p2p::sharing::announce_peer(11420, Vec::new()).await;
                });
            }

            // ── Continuous training subsystems (collector, evaluation, self-play) ──
            let training_collector = unified_training_collector::UnifiedTrainingCollector::new(20_000);
            let eval_harness = evaluation_harness::EvaluationHarness::new(orchestrator.clone());
            let forgetting = forgetting_prevention::ForgettingPrevention::new(eval_harness.clone(), orchestrator.clone());
            let adapters_dir = bonsai_home.join("adapters");
            let adapter_registry = promotion_gate::AdapterRegistry::new(adapters_dir.clone());
            let promotion_gate = promotion_gate::PromotionGate::new(eval_harness.clone(), adapter_registry.clone(), orchestrator.clone());
            let self_play_trainer = eternal_training_loop::SelfPlayTrainer::new(orchestrator.clone(), training_collector.clone());
            let eternal_loop = eternal_training_loop::EternalTrainingLoop::new(
                training_collector.clone(),
                eval_harness.clone(),
                forgetting.clone(),
                promotion_gate.clone(),
                self_play_trainer.clone(),
                orchestrator.clone(),
                early_knowledge.clone(),
            );
            // Start background eternal training loop
            eternal_loop.clone().spawn();
            let training_state = Arc::new(training_commands::TrainingState::new(
                training_collector.clone(),
                eternal_loop.clone(),
                adapter_registry.clone(),
            ));

            // Initialize Universal Capability Registry (UCR) and register tool registry
            let capability_registry = bonsai_capability_registry::UniversalCapabilityRegistry::new();
            // Register the current ToolRegistry snapshot as a capability source
            tauri::async_runtime::block_on(capability_registry.register(Box::new((*tool_registry).clone())));

            // ── UCR startup validation ────────────────────────────────────────────
            // Verify every tool in built_in_tools() is present in the merged tool list.
            // This catches future "invisible tool" bugs at startup rather than at runtime.
            {
                let builtin_names: Vec<String> = crate::tools::built_in_tools()
                    .into_iter().map(|t| t.name).collect();
                let registry_names: std::collections::HashSet<String> =
                    tool_registry.list_tools().into_iter().map(|t| t.name).collect();
                let merged = crate::tools::all_tools_full(None, None, Some(&tool_registry));
                let merged_names: std::collections::HashSet<String> =
                    merged.iter().map(|t| t.name.clone()).collect();
                for name in &builtin_names {
                    if !merged_names.contains(name.as_str()) {
                        tracing::warn!("[UCR] built-in tool '{}' is NOT in the merged tool list — it will be invisible to the ReAct loop", name);
                    }
                }
                for name in &registry_names {
                    if !merged_names.contains(name.as_str()) {
                        tracing::warn!("[UCR] registry tool '{}' is NOT in the merged tool list — it will be invisible to the ReAct loop", name);
                    }
                }
                tracing::info!("[UCR] startup validation: {} built-ins + {} registry tools → {} merged tools",
                    builtin_names.len(), registry_names.len(), merged_names.len());
            }

            // Thoughts DB — persist model "thought" segments for auditing and UI
            let thoughts_db = Arc::new(
                tauri::async_runtime::block_on(thoughts::ThoughtsStore::new(wal.pool()))
                    .expect("Thoughts DB init failed"),
            );

            // CAS — content-addressed blob store backed by SQLite + flat files
            let cas_data_dir = app_handle.path().app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let cas_store = Arc::new(
                tauri::async_runtime::block_on(
                    bonsai_cas::CasStore::open(
                        &cas_data_dir.join("cas.db"),
                        &cas_data_dir.join("cas_blobs"),
                    )
                ).expect("CAS store init failed")
            );

            // Actor system — supervision trees for swarm and background agents
            let actor_system = bonsai_actors::ActorSystem::new();

            // Federated training coordinator
            let machine_id = format!("local-{}", uuid::Uuid::new_v4().simple());
            let federated_trainer = crate::federated_trainer::FederatedTrainer::new(machine_id);

            // OmnAI OS layer
            let early_omnipresent = crate::omnipresent_capture::OmnipresentCapture::new(
                cas_store.clone(),
                training_collector.clone(),
            );
            crate::omnipresent_capture::spawn_hardware_sampler(early_omnipresent.clone());
            let early_predictive = crate::predictive_engine::PredictiveEngine::new();
            let early_omnfs = crate::omnfs::OmnFS::new(cas_store.clone());

            // OmniDesktop and ProcessManager (don't need Sylva)
            let early_omni_desktop = crate::omni_desktop::OmniDesktop::new(gpu_ctrl_inst.clone());
            let early_process_manager = crate::process_manager::ProcessManager::new(gpu_ctrl_inst.clone());

            // Sylva scripting runtime — hot-reloadable Lua tools
            let scripts_dir = cas_data_dir.join("scripts");
            let sylva = tauri::async_runtime::block_on(
                crate::sylva::SylvaState::new(
                    tool_registry.clone(),
                    scripts_dir,
                    app_handle.clone(),
                )
            ).unwrap_or_else(|e| {
                tracing::warn!("[sylva] init failed (Lua unavailable?): {e}");
                // Return a dummy state so the app still starts
                panic!("Sylva init failed: {e}")
            });

            // OmniShell — needs Sylva runtime
            let early_omni_shell = Arc::new(crate::omni_shell::OmniShellState::new(
                early_predictive.clone(),
                sylva.runtime.clone(),
                early_omnipresent.clone(),
            ));

            // Phase 3 OS: OmniSession, DeviceManager, OmniBoot
            let early_omni_session = crate::omni_session::OmniSession::new(
                early_omni_desktop.clone(),
                early_omni_shell.clone(),
                early_process_manager.clone(),
                early_omnipresent.clone(),
                early_predictive.clone(),
                cas_store.clone(),
                auth_state.clone(),
            );
            let early_device_manager = crate::device_manager::DeviceManager::new();
            let early_omni_boot = crate::omni_boot::OmniBoot::new(cas_store.clone());

            app.manage(AppState {
                orchestrator:     orchestrator.clone(),
                whisper:          whisper.clone(),
                wal,
                chat_sessions:    chat_sessions.clone(),
                pty_sessions,
                // Workstream states (A-D)
                auth_state: auth_state.clone(),
                market_state: market_state.clone(),
                meeting_agent: meeting_agent.clone(),
                continuous_trainer: continuous_trainer.clone(),
                training: training_state.clone(),
                bootstrap_cancel: bootstrap_cancel.clone(),
                chat_cancel:      chat_cancel.clone(),
                voice_cancel:     voice_cancel.clone(),
                agent_connect,
                ws_router:          ws_router.clone(),
                pair_token:         pair_token.clone(),
                agent_store,
                swarm_orchestrator: swarm_orch,
                swarm_cancels,
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
                bonsai_core: shared_bonsai_core,
                telemetry:   telemetry_store.clone(),
                hybrid_engine: Arc::new(hybrid_engine::HybridEngineState::new()),
                gpu: Arc::new(gpu_layer::GpuLayer::new(&gpu_layer::GpuLayer::detect())),
                dual_session: shared_dual_session.clone(),
                training_loop: Arc::new(training_loop::TrainingLoopState::new(
                    orchestrator.clone(),
                    telemetry_store,
                )),
                self_play:     self_play_state,
                plugin_host,
                tool_registry,
                capability_registry: capability_registry.clone(),
                thoughts_db: thoughts_db.clone(),
                thinking_settings: Arc::new(tokio::sync::RwLock::new(serde_json::json!({
                    "show_primary_thinking": true,
                    "show_draft_thinking": true,
                    "show_micro_thinking": false,
                    "show_critic_thinking": true,
                    "show_tool_rationale": true,
                    "show_swarm_thinking": false,
                    "max_thinking_tokens": 2048
                }))),
                cross_training: cross_training_sender,
                mcp_port,
                tool_watcher: tool_watcher_state,
                shared_arena,
                micro_bonsai,
                swarm_config_store,
                gpu_controller: gpu_controller.clone(),
                skill_registry,
                cas_store,
                sylva,
                federated_trainer,
                actor_system,
                game_sessions: early_game_sessions.clone(),
                knowledge: early_knowledge.clone(),
                reasoning: early_reasoning.clone(),
                belief_reviser: early_belief_reviser.clone(),
                metacognitive: early_metacognitive.clone(),
                omnipresent: early_omnipresent.clone(),
                predictive_engine: early_predictive.clone(),
                omnfs: early_omnfs.clone(),
                omni_desktop: early_omni_desktop.clone(),
                omni_shell: early_omni_shell,
                process_manager: early_process_manager.clone(),
                omni_session: early_omni_session,
                device_manager: early_device_manager,
                omni_boot: early_omni_boot,
            });
            app.manage(remote_manager.clone());
            app.manage(features::FeatureFlags::global());

            // ── Start Copilot Orchestrator (local REST control) ─────────────
            {
                let ah = app_handle.clone();
                let gc = gpu_controller.clone();
                let mo = orchestrator.clone();
                tauri::async_runtime::spawn(async move {
                    // Best-effort: start the local orchestrator on 127.0.0.1:11380
                    orchestrator::start_orchestrator(ah, gc, mo).await;
                });
            }

            // ── Forward training-loop progress events to the frontend ─────────
            {
                use tauri::Manager;
                let app_state = app.state::<AppState>();
                let mut rx = app_state.training_loop.subscribe_progress();
                let bh = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    while let Ok(payload) = rx.recv().await {
                        let _ = bh.emit("training-loop-progress", payload);
                    }
                });
            }

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
            // ── Workstreams (Auth, Marketplace, Meeting, Training) ─────────────
            auth_commands::create_profile,
            auth_commands::unlock_profile,
            auth_commands::lock_profile,
            auth_commands::create_workspace,
            auth_commands::share_workspace,
            auth_commands::list_workspaces,
            marketplace_commands::publish_asset,
            marketplace_commands::search_marketplace,
            marketplace_commands::install_asset,
            meeting_agent::start_meeting_agent,
            meeting_agent::stop_meeting_agent,
            meeting_agent::pause_meeting_agent,
            meeting_agent::ask_meeting_agent,
            meeting_agent::get_meeting_transcript,
            meeting_agent::get_meeting_notes,
            meeting_agent::get_meeting_action_items,
            meeting_agent::update_action_item,
            meeting_agent::get_meeting_summary,
            meeting_agent::is_meeting_running,
            continuous_training::ingest_feedback_continuous,
            continuous_training::continuous_training_status,
            continuous_training::trigger_training,
            // Training UI / loop commands
            training_commands::get_training_stats,
            training_commands::get_training_examples,
            training_commands::delete_training_example,
            training_commands::edit_training_example,
            training_commands::boost_training_example,
            training_commands::bulk_delete_training_data,
            training_commands::export_training_data,
            training_commands::wipe_training_database,
            training_commands::trigger_training_cycle,
            training_commands::get_evaluation_results,
            training_commands::get_ciq_history,
            training_commands::get_alerts,
            training_commands::run_core_competency_check,
            training_commands::get_curriculum_status,
            training_commands::rollback_adapter,
            training_commands::set_training_preferences,
            training_commands::get_training_preferences,
            training_commands::get_self_play_state,
            training_commands::get_forgetting_baseline,
            training_commands::get_training_loop_history,
            training_commands::ingest_feedback_ui,
            training_commands::ingest_edit,
            training_commands::train_reasoning,
            // ── OmnAI OS ──────────────────────────────────────────────────────
            omnipresent_capture::omn_record_event,
            omnipresent_capture::omn_get_recent_events,
            omnipresent_capture::omn_get_session_summary,
            omnipresent_capture::omn_update_context,
            omnipresent_capture::omn_new_session,
            predictive_engine::get_predictions,
            predictive_engine::get_automation_rules,
            predictive_engine::get_pending_suggestions,
            predictive_engine::approve_automation,
            predictive_engine::reject_automation,
            predictive_engine::add_automation_rule,
            predictive_engine::delete_automation_rule,
            omnfs::omnfs_read,
            omnfs::omnfs_write,
            omnfs::omnfs_delete,
            omnfs::omnfs_stat,
            omnfs::omnfs_list_dir,
            omnfs::omnfs_snapshot,
            omnfs::omnfs_rollback,
            omnfs::omnfs_list_snapshots,
            omnfs::omnfs_stats,
            // ── OmniDesktop ───────────────────────────────────────────────────
            omni_desktop::omni_window_list,
            omni_desktop::omni_window_open,
            omni_desktop::omni_window_close,
            omni_desktop::omni_window_focus,
            omni_desktop::omni_window_move,
            omni_desktop::omni_window_minimize,
            omni_desktop::omni_window_maximize,
            omni_desktop::omni_window_restore,
            omni_desktop::omni_desktop_layout,
            omni_desktop::omni_desktop_wallpaper_set,
            omni_desktop::omni_panel_list,
            omni_desktop::omni_panel_add,
            omni_desktop::omni_panel_widget_add,
            omni_desktop::omni_desktop_damage,
            omni_desktop::omni_workspace_switch,
            // ── OmniShell ─────────────────────────────────────────────────────
            omni_shell::omni_shell_exec,
            omni_shell::omni_shell_predict,
            omni_shell::omni_shell_history,
            omni_shell::omni_shell_nl,
            omni_shell::omni_shell_alias_set,
            omni_shell::omni_shell_alias_delete,
            omni_shell::omni_shell_aliases,
            omni_shell::omni_shell_config_set,
            omni_shell::omni_shell_cwd,
            // ── Process Manager ───────────────────────────────────────────────
            process_manager::omni_process_list,
            process_manager::omni_process_spawn,
            process_manager::omni_process_kill,
            process_manager::omni_process_priority,
            process_manager::omni_process_optimize,
            process_manager::omni_process_tree,
            process_manager::omni_process_stats,
            // ── OmniSession ───────────────────────────────────────────────────
            omni_session::omni_session_login,
            omni_session::omni_session_logout,
            omni_session::omni_session_summary,
            omni_session::omni_session_snapshot,
            omni_session::omni_session_state,
            // ── Device Manager ────────────────────────────────────────────────
            device_manager::omni_devices_list,
            device_manager::omni_display_config,
            device_manager::omni_audio_device_set,
            device_manager::omni_input_device_info,
            device_manager::omni_device_optimize,
            device_manager::omni_device_hotplug,
            // ── OmniBoot ──────────────────────────────────────────────────────
            omni_boot::omni_boot_verify,
            omni_boot::omni_boot_manifest,
            omni_boot::omni_boot_snapshot,
            omni_boot::omni_boot_report,
            omni_boot::omni_boot_load_manifest,
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
            commands::get_mcp_port,
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
            // Capability registry queries
            capability_commands::get_capability_summary,
            capability_commands::get_capability_manifest,
            capability_commands::query_capabilities,
            // Thoughts persistence commands
            thoughts_commands::add_thought,
            thoughts_commands::get_thoughts_for_turn,
            thoughts_commands::clear_thoughts_for_session,
            thoughts_commands::search_thinking_history,
            thoughts_commands::record_thinking,
            thoughts_commands::get_thinking_settings,
            thoughts_commands::set_thinking_settings,
            // ── Sylva scripting ───────────────────────────────────────────────
            sylva::sylva_exec,
            sylva::sylva_exec_file,
            sylva::sylva_list_scripts,
            sylva::get_sylva_history,
            sylva::sylva_clear_history,
            sylva::sylva_load_script,
            sylva::sylva_get_script_content,
            sylva::sylva_save_script,
            // ── Federated training ────────────────────────────────────────────
            federated_trainer::federated_stats,
            federated_trainer::federated_list_adapters,
            // ── Chess & Go ────────────────────────────────────────────────────
            games::create_chess_game,
            games::make_chess_move,
            games::get_chess_game,
            games::resign_chess_game,
            games::list_chess_games,
            games::create_go_game,
            games::make_go_move,
            games::get_go_game,
            games::resign_go_game,
            games::list_go_games,
            games::export_go_sgf,
            games::create_tournament,
            games::get_tournament_standings,
            games::list_tournaments,
            games::get_daily_puzzle,
            games::check_puzzle_move,
            games::chess_ai_move,
            games::go_ai_move,
            games::export_chess_pgn,
            games::spectate_game,
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
            // ── BonsAI-Core ───────────────────────────────────────────────────
            commands::start_training_cycle,
            commands::get_training_status,
            commands::get_training_history,
            // ── Native GPU engine ─────────────────────────────────────────────
            commands::load_model_native,
            commands::apply_lora_native,
            commands::get_memory_status,
            commands::load_model_gpu,
            commands::compare_models,
            commands::start_training_loop,
            commands::stop_training_loop,
            commands::get_training_loop_status,
            // ── Rich Markdown / Multi-modal ───────────────────────────────────
            rich_markdown::render_rich_block,
            sandbox_executor::run_sandboxed_code,
            image_generation::generate_image_command,
            commands::generate_music_command,
            tts_engine::tts_synthesize,
            plugin_loader::list_plugins_cmd,
            plugin_loader::load_plugin_cmd,
            // Self-play training
            self_play::start_self_play,
            self_play::stop_self_play,
            self_play::get_self_play_status,
            // Plugin host
            plugin_host::list_loaded_plugins,
            plugin_host::load_plugin_from_dir,
            plugin_host::execute_plugin,
            // Tool registry
            tool_registry::list_tools,
            tool_registry::run_tool,
            // GPU crash flag
            commands::get_gpu_crash_flag,
            commands::clear_gpu_crash_flag,
            // Tool composition DSL
            tool_compose::validate_composed_skill,
            // Micro BonsAI model selector
            micro_bonsai::micro_select_model,
            micro_bonsai::micro_hardware_snapshot,
            micro_bonsai::micro_perf_history,
            // Custom swarm configurations
            swarm_config::create_swarm_config,
            swarm_config::list_swarm_configs,
            swarm_config::get_swarm_config,
            swarm_config::update_swarm_config,
            swarm_config::delete_swarm_config,
            swarm_config::activate_swarm,
            swarm_config::arena_stats,
            // GPU controller
            gpu_controller::get_gpu_controller_health,
            gpu_controller::reset_gpu_controller,
            // Multi-modal Phase 1 — Kokoro TTS
            multimodal::kokoro::kokoro_synthesize,
            multimodal::kokoro::list_kokoro_voices,
            multimodal::kokoro::kokoro_available,
            // Multi-modal Phase 1 — Depth estimation
            multimodal::depth::estimate_depth,
            multimodal::depth::depth_model_available,
            // Multi-modal Phase 1 — YOLO detection
            multimodal::yolo::detect_objects_cmd,
            multimodal::yolo::detect_chart_patterns,
            multimodal::yolo::yolo_available,
            // Multi-modal Phase 2 — OpenCV 4.12 vision toolkit
            multimodal::opencv_tools::opencv_run_op,
            multimodal::opencv_tools::opencv_available,
            multimodal::opencv_tools::opencv_detect_faces,
            multimodal::opencv_tools::opencv_detect_edges,
            multimodal::opencv_tools::opencv_pipeline_cmd,
            // Multi-modal Phase 2 — PixAI image tagger
            multimodal::pixai_tagger::pixai_tag,
            multimodal::pixai_tagger::pixai_available,
            // Multi-modal Phase 2 — NuMarkdown document OCR
            multimodal::nu_markdown::image_to_markdown,
            multimodal::nu_markdown::numarkdown_model_path,
            multimodal::nu_markdown::numarkdown_mmproj_path,
            // Multi-modal Phase 2 — Qwen image editing
            multimodal::image_edit::edit_image_cmd,
            multimodal::image_edit::generate_image_rapid_cmd,
            multimodal::image_edit::generate_multiview_cmd,
            multimodal::image_edit::image_edit_available,
            // Multi-modal Phase 2 — Sulphur-2 video generation
            multimodal::video_gen::generate_video_cmd,
            multimodal::video_gen::video_gen_available,
            // Multi-modal Phase 2 — TRELLIS.2-4B 3D generation
            multimodal::threed_gen::generate_3d_model_cmd,
            multimodal::threed_gen::generate_3d_from_text_cmd,
            multimodal::threed_gen::threed_gen_available,
            // Vision oracle self-play training loop
            vision_training::start_vision_training,
            vision_training::vision_training_oracles_available,
            // Skills.sh skill registry
            skill_registry::list_installed_skills,
            skill_registry::install_skill_local,
            skill_registry::install_skill_content,
            skill_registry::install_skill_from_skills_sh,
            skill_registry::toggle_skill,
            skill_registry::uninstall_skill,
            skill_registry::search_skills_marketplace,
            skill_registry::export_tool_as_skill,
            skill_registry::get_skill_context_for_prompt,
            skill_registry::verify_skill_integrity,
            skill_compiler_commands::compile_skill_from_path,
            skill_compiler_commands::compile_skill_from_content,
            skill_compiler_commands::compile_and_register_skill,
            skill_compiler_commands::verify_compiled_skill,
            skill_compiler_commands::list_compiled_skills,
            skill_compiler_commands::invoke_skill,
            skill_compiler_commands::distill_skill_to_lora,
            skill_compiler_commands::uninstall_compiled_skill,
            marketplace_commands::publish_compiled_skill_to_marketplace,
            marketplace_commands::discover_peer_skills,
            marketplace_commands::install_skill_from_marketplace,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
