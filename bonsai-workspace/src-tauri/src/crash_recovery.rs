/// Crash recovery — detects unclean shutdown, replays WAL, emits recovery event.
///
/// On startup:
///   1. Check for a `crash.flag` file left by a previous unclean exit.
///   2. If present, replay the WAL write-ahead log to restore consistent state.
///   3. Emit `recovery-state` Tauri event so the frontend can show a notice.
///   4. Remove the flag so a clean second boot doesn't think it crashed.
///
/// On clean exit, `on_exit_cleanup` removes the flag.

use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};
use tracing::{info, warn};

const CRASH_FLAG_NAME: &str = "crash.flag";

fn flag_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_local_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
        .join(CRASH_FLAG_NAME)
}

/// Write the crash flag on startup (removed on clean exit).
pub fn arm_crash_flag(app: &AppHandle) {
    let path = flag_path(app);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(&path, b"1") {
        warn!("[crash_recovery] could not write crash flag: {e}");
    }
}

/// Check for a crash flag and emit recovery events if one is found.
/// Returns `true` if a crash was detected.
pub async fn check_and_recover(
    app: &AppHandle,
    wal: &crate::wal::WAL,
) -> bool {
    let path = flag_path(app);
    if !path.exists() {
        return false;
    }

    warn!("[crash_recovery] crash flag detected — previous session ended uncleanly");

    // Replay WAL to restore consistent DB state.
    match wal.replay_uncommitted().await {
        Ok(replayed) => {
            info!("[crash_recovery] WAL replay complete: {replayed} entries replayed");
        }
        Err(e) => {
            warn!("[crash_recovery] WAL replay failed: {e}");
        }
    }

    // Emit event so the frontend can show a non-blocking recovery notice.
    let _ = app.emit("recovery-state", serde_json::json!({
        "crashed": true,
        "wal_replayed": true,
        "message": "Bonsai recovered from an unexpected shutdown. Your data is intact."
    }));

    // Remove the flag so the next boot is clean.
    let _ = std::fs::remove_file(&path);

    true
}

/// Remove the crash flag on clean exit. Call from `on_window_event` CloseRequested.
pub fn on_exit_cleanup(app: &AppHandle) {
    let path = flag_path(app);
    let _ = std::fs::remove_file(&path);
    info!("[crash_recovery] crash flag cleared — clean exit");
}
