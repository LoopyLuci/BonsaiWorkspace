//! Zero-downtime hot model reload.
//!
//! Watches `~/.bonsai/models/bonsai-latest.gguf` (or any configured path) for
//! changes.  When a new file appears or the mtime advances, it:
//!   1. Triggers a registry refresh so the orchestrator sees the new GGUF.
//!   2. Loads the new model ID into a FREE slot (old slot keeps serving).
//!   3. Waits for the new slot to become Ready.
//!   4. Unloads the old slot (new requests already route to the new slot).
//!   5. Emits a `model-reloaded` event to all Tauri windows (frontend toast).
//!
//! The watcher polls every 2 s — lightweight, no inotify dependency needed on
//! Windows where ReadDirectoryChangesW is less portable across MSVC/MinGW.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use tracing::{error, info};

use std::sync::Arc;

use crate::model_orchestrator::ModelOrchestrator;

const POLL_INTERVAL: Duration = Duration::from_secs(2);
/// After loading the new slot, wait this long for in-flight requests on the
/// old slot to drain before killing it.
const DRAIN_GRACE: Duration = Duration::from_secs(5);

#[derive(Clone, serde::Serialize)]
struct ModelReloadedPayload {
    model_id: String,
    path: String,
    status: String,
}

/// Spawn the background watcher.  Call once from `lib.rs` setup after the
/// orchestrator is available in AppState.
pub fn spawn(app: AppHandle, orchestrator: Arc<ModelOrchestrator>, watch_path: PathBuf) {
    tauri::async_runtime::spawn(async move {
        watch_loop(app, orchestrator, watch_path).await;
    });
}

async fn watch_loop(app: AppHandle, orchestrator: Arc<ModelOrchestrator>, path: PathBuf) {
    // Seed mtime from the current file so we don't trigger a spurious reload on startup.
    let mut last_mtime: Option<SystemTime> = std::fs::metadata(&path)
        .ok()
        .and_then(|m| m.modified().ok());
    let mut last_model_id: Option<String> = None;

    loop {
        tokio::time::sleep(POLL_INTERVAL).await;

        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        let Ok(mtime) = meta.modified() else { continue };

        if last_mtime.map_or(true, |prev| mtime > prev) {
            last_mtime = Some(mtime);

            // Derive a model ID from the filename (registry key = stem).
            let model_id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("bonsai-latest")
                .to_string();

            // Skip if it's the same model id that's already loaded.
            if last_model_id.as_deref() == Some(&model_id) {
                continue;
            }

            info!("[hot_reload] change detected: {}", path.display());

            // Refresh registry so the orchestrator sees the new file.
            orchestrator.refresh_registry();

            // Load into a free slot (non-blocking kick-off).
            let rx = orchestrator.load(model_id.clone());

            // Await readiness (up to the orchestrator's own load timeout).
            match rx.await {
                Ok(Ok(())) => {
                    info!("[hot_reload] new model ready: {model_id}");

                    // Drain old slot.
                    if let Some(old_id) = last_model_id.take() {
                        tokio::time::sleep(DRAIN_GRACE).await;
                        orchestrator.unload_model(&old_id).await;
                        info!("[hot_reload] old model unloaded: {old_id}");
                    }

                    last_model_id = Some(model_id.clone());

                    let _ = app.emit(
                        "model-reloaded",
                        ModelReloadedPayload {
                            model_id: model_id.clone(),
                            path: path.display().to_string(),
                            status: "ready".into(),
                        },
                    );

                    // System-tray notification
                    let _ = app
                        .notification()
                        .builder()
                        .title("🧠 BonsAI Updated")
                        .body("A new brain update is live. BonsAI just got smarter!")
                        .show();
                }
                Ok(Err(e)) => {
                    error!("[hot_reload] failed to load {model_id}: {e}");
                    let _ = app.emit(
                        "model-reloaded",
                        ModelReloadedPayload {
                            model_id,
                            path: path.display().to_string(),
                            status: format!("error: {e}"),
                        },
                    );
                }
                Err(_) => {
                    error!("[hot_reload] orchestrator dropped load receiver");
                }
            }
        }
    }
}
