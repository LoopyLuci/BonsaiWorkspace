use std::sync::Arc;
use std::time::Duration;
use sysinfo::System;
use tracing::{debug, info, warn};

use crate::checkpoint_impl;
use crate::state::DaemonState;
use cas::{CasEvent, CasStore};

/// Memory ceiling above which a warning is emitted (bytes).
const MEMORY_WARN_BYTES: u64 = 512 * 1024 * 1024; // 512 MiB
/// Checkpoint interval (periodic fallback even without CAS events).
const CHECKPOINT_INTERVAL: Duration = Duration::from_secs(120);
/// Health poll interval.
const POLL_INTERVAL: Duration = Duration::from_secs(30);

/// Runs in a background task. Every `POLL_INTERVAL`:
/// - logs daemon RSS; warns if above threshold
/// Every `CHECKPOINT_INTERVAL` (or on CAS write events):
/// - writes a CAS checkpoint of the transfer state
pub async fn run_health_monitor(state: Arc<DaemonState>, cas: Arc<CasStore>) {
    let mut sys = System::new();
    let pid = sysinfo::get_current_pid().ok();
    let mut ticks: u32 = 0;

    // On startup, attempt to replay the most recent checkpoint if a key was persisted.
    // The checkpoint key is written to the platform data dir on each successful checkpoint.
    let base = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("bonsai");
    let checkpoint_key_path = base.join("last_checkpoint.key");
    if checkpoint_key_path.exists() {
        if let Ok(key_str) = std::fs::read_to_string(&checkpoint_key_path) {
            let key_str = key_str.trim();
            if !key_str.is_empty() {
                match cas::CasKey::from_hex(key_str) {
                    Ok(key) => match checkpoint_impl::restore(&state, &cas, &key).await {
                        Ok(()) => {
                            info!(cas_key = %key_str, "health-monitor: restored state from checkpoint")
                        }
                        Err(e) => {
                            warn!(error = %e, "health-monitor: checkpoint restore failed (ignored)")
                        }
                    },
                    Err(e) => warn!(
                        "health-monitor: invalid checkpoint key in {checkpoint_key_path:?}: {e}"
                    ),
                }
            }
        }
    }

    // Subscribe to CAS write events so we can trigger reactive checkpoints.
    let mut cas_watch = cas.watch();

    loop {
        tokio::select! {
            // ── Periodic tick ────────────────────────────────────────────────
            _ = tokio::time::sleep(POLL_INTERVAL) => {
                ticks += 1;

                // RSS check
                sys.refresh_memory();
                if let Some(pid) = pid {
                    sys.refresh_process(pid);
                    if let Some(proc) = sys.process(pid) {
                        let rss = proc.memory();
                        if rss > MEMORY_WARN_BYTES {
                            warn!(
                                rss_mib = rss / (1024 * 1024),
                                threshold_mib = MEMORY_WARN_BYTES / (1024 * 1024),
                                "bonsai-daemon memory above threshold"
                            );
                        } else {
                            info!(rss_mib = rss / (1024 * 1024), "health-monitor: ok");
                        }
                    }
                }

                // Periodic checkpoint (fallback when no CAS events fire)
                let checkpoint_every =
                    (CHECKPOINT_INTERVAL.as_secs() / POLL_INTERVAL.as_secs()) as u32;
                if ticks % checkpoint_every == 0 {
                    match checkpoint_impl::checkpoint(&state, &cas).await {
                        Ok(key) => {
                            info!(cas_key = %key, "health-monitor: periodic checkpoint");
                            let _ = std::fs::write(&checkpoint_key_path, key.to_string());
                        }
                        Err(e)  => warn!(error = %e, "health-monitor: checkpoint failed"),
                    }
                }
            }

            // ── CAS watch — reactive checkpoint on significant writes ─────────
            event = cas_watch.recv() => {
                match event {
                    Ok(CasEvent::Inserted { key, size, .. }) if size > 1024 => {
                        debug!(cas_key = %key, "health-monitor: CAS insert → checkpoint");
                        match checkpoint_impl::checkpoint(&state, &cas).await {
                            Ok(ck) => {
                                info!(cas_key = %ck, "health-monitor: reactive checkpoint");
                                let _ = std::fs::write(&checkpoint_key_path, ck.to_string());
                            }
                            Err(e) => warn!(error = %e, "health-monitor: reactive checkpoint failed"),
                        }
                    }
                    Err(_) => {
                        // Lagged receiver — resubscribe.
                        cas_watch = cas.watch();
                    }
                    _ => {}
                }
            }
        }
    }
}
