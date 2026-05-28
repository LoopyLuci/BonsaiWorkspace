use std::sync::Arc;
use std::time::Duration;
use sysinfo::System;
use tracing::{info, warn};

use bonsai_cas::CasStore;
use crate::state::DaemonState;
use crate::checkpoint_impl;

/// Memory ceiling above which a warning is emitted (bytes).
const MEMORY_WARN_BYTES: u64 = 512 * 1024 * 1024; // 512 MiB
/// Checkpoint interval.
const CHECKPOINT_INTERVAL: Duration = Duration::from_secs(120);
/// Health poll interval.
const POLL_INTERVAL: Duration = Duration::from_secs(30);

/// Runs in a background task. Every `POLL_INTERVAL`:
/// - logs daemon RSS; warns if above threshold
/// Every `CHECKPOINT_INTERVAL`:
/// - writes a CAS checkpoint of the transfer state
pub async fn run_health_monitor(state: Arc<DaemonState>, cas: Arc<CasStore>) {
    let mut sys = System::new();
    let pid = sysinfo::get_current_pid().ok();
    let mut ticks: u32 = 0;

    loop {
        tokio::time::sleep(POLL_INTERVAL).await;
        ticks += 1;

        // ── RSS check ────────────────────────────────────────────────────────
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

        // ── Periodic checkpoint ──────────────────────────────────────────────
        let checkpoint_every = (CHECKPOINT_INTERVAL.as_secs() / POLL_INTERVAL.as_secs()) as u32;
        if ticks % checkpoint_every == 0 {
            match checkpoint_impl::checkpoint(&state, &cas).await {
                Ok(key) => info!(cas_key = %key, "health-monitor: checkpoint written"),
                Err(e)  => warn!(error = %e, "health-monitor: checkpoint failed"),
            }
        }
    }
}
