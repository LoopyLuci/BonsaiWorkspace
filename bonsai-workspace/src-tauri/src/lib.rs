mod action_parser;
mod agent_connect;
mod agent_store;
mod api_server;
mod bootstrap;
mod chat_sessions;
mod commands;
mod config;
mod model_orchestrator;
mod model_registry;
mod remote;
mod remote_input;
mod sidecar_manager;
mod swarm_orchestrator;
mod tools;
mod wal;
mod ws_router;

use std::collections::HashMap;
use std::sync::{
    atomic::AtomicBool,
    Arc,
    Mutex as StdMutex,
};
use tauri::Emitter;
use tokio::sync::Mutex;

pub struct PtySession {
    pub writer: Box<dyn std::io::Write + Send>,
    pub master: Box<dyn portable_pty::MasterPty + Send>,
}

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
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init());

    #[cfg(any(target_os = "android", target_os = "ios"))]
    let builder = builder.plugin(tauri_plugin_barcode_scanner::init());

    builder
        .setup(move |app| {
            use tauri::Manager;
            let app_handle = app.handle().clone();

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

            // Model orchestrator — starts event loop immediately;
            // slots go to Crashed if llama-server isn't present yet.
            let orchestrator = Arc::new(
                model_orchestrator::ModelOrchestrator::new(app_handle.clone()),
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
            });
            app.manage(remote_manager.clone());

            // Spawn the OpenAI-compatible API server on the configured host/port.
            let api_config = config::load_config(&app_handle).unwrap_or_default();
            {
                let orch   = orchestrator.clone();
                let remote = remote_manager.clone();
                let ws     = ws_router.clone();
                let token  = pair_token.clone();
                let host   = api_config.api_host.clone();
                let port   = api_config.api_port;
                tauri::async_runtime::spawn(async move {
                    api_server::start(orch, remote, ws, token, host, port).await;
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
                                Err(e) => eprintln!("[mdns] service info error: {e}"),
                            }
                        }
                        Err(e) => eprintln!("[mdns] daemon error: {e}"),
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
                            eprintln!("[bootstrap] Failed: {e}");
                            let _ = bh.emit("bootstrap-error", e.to_string());
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // ── File system ───────────────────────────────────────────────────
            commands::read_file,
            commands::write_file,
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
            commands::save_chat_session,
            commands::load_chat_session,
            commands::delete_chat_session,
            commands::rename_chat_session,
            commands::duplicate_chat_session,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
