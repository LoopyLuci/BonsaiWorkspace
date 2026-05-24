//! Generic process supervisor for llama-server slots (and similar sidecars).
//!
//! `SidecarSupervisor` wraps a `std::process::Child`, polls its HTTP health
//! endpoint on a background task, and broadcasts state changes via a
//! `tokio::sync::watch` channel.  The orchestrator subscribes to the channel
//! so it can react to Ready / Crashed transitions without spawning a separate
//! health-poll future per slot.

use std::time::Duration;
use reqwest::Client;
use tokio::sync::watch;

/// Current health state of a supervised sidecar process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidecarStatus {
    /// Process is spawned; health probe has not yet succeeded.
    Loading,
    /// Health endpoint returned 200 — the sidecar is ready to serve requests.
    Ready,
    /// Process exited or the health probe timed out.
    Crashed { code: Option<i32>, detail: String },
}

/// Configuration passed to [`SidecarSupervisor::start`].
pub struct SidecarConfig {
    /// HTTP base URL of the sidecar (`http://127.0.0.1:{port}`).
    pub base_url:           String,
    /// Path to probe for health (`/health` or `/v1/models`).
    pub health_path:        String,
    /// Maximum total time to wait for the first successful health probe.
    pub load_timeout:       Duration,
    /// Interval between consecutive health probes during startup.
    pub poll_interval:      Duration,
    /// Log file for stderr capture (already opened by the orchestrator).
    /// Supply `None` to discard stderr.
    pub log_path:           Option<std::path::PathBuf>,
}

impl Default for SidecarConfig {
    fn default() -> Self {
        Self {
            base_url:      "http://127.0.0.1:8080".into(),
            health_path:   "/health".into(),
            load_timeout:  Duration::from_secs(300),
            poll_interval: Duration::from_millis(500),
            log_path:      None,
        }
    }
}

/// Owns the child process and broadcasts its health state.
pub struct SidecarSupervisor {
    /// Broadcast channel — callers clone the receiver to watch status.
    pub status_tx: watch::Sender<SidecarStatus>,
    pub status_rx: watch::Receiver<SidecarStatus>,
    /// The supervised child process.
    child: std::sync::Mutex<Option<std::process::Child>>,
}

impl SidecarSupervisor {
    /// Spawn `child` and start the background health-poll task.
    /// Returns a supervisor whose `status_rx` will transition from
    /// `Loading → Ready` or `Loading → Crashed`.
    pub fn start(child: std::process::Child, cfg: SidecarConfig) -> Self {
        let (tx, rx)  = watch::channel(SidecarStatus::Loading);
        let supervisor = Self {
            status_tx: tx.clone(),
            status_rx: rx,
            child:     std::sync::Mutex::new(Some(child)),
        };

        // Background task: probe health until ready or timeout
        let base_url     = cfg.base_url.clone();
        let health_path  = cfg.health_path.clone();
        let poll_interval = cfg.poll_interval;
        let load_timeout  = cfg.load_timeout;

        tokio::spawn(async move {
            let client    = Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap_or_default();
            let url       = format!("{}{}", base_url.trim_end_matches('/'), health_path);
            let deadline  = tokio::time::Instant::now() + load_timeout;

            loop {
                if tokio::time::Instant::now() >= deadline {
                    let _ = tx.send(SidecarStatus::Crashed {
                        code:   None,
                        detail: format!(
                            "health probe timed out after {}s",
                            load_timeout.as_secs()
                        ),
                    });
                    return;
                }

                tokio::time::sleep(poll_interval).await;

                match client.get(&url).send().await {
                    Ok(r) if r.status().is_success() => {
                        let _ = tx.send(SidecarStatus::Ready);
                        return;
                    }
                    _ => {}
                }
            }
        });

        supervisor
    }

    /// Kill the supervised process if it is still running.
    pub fn kill(&self) {
        if let Ok(mut guard) = self.child.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
            }
        }
    }

    /// Non-blocking check: has the child already exited?
    pub fn try_wait(&self) -> Option<std::process::ExitStatus> {
        if let Ok(mut guard) = self.child.lock() {
            if let Some(child) = guard.as_mut() {
                return child.try_wait().ok().flatten();
            }
        }
        None
    }

    /// Current status snapshot (cheap — reads the watch channel).
    pub fn status(&self) -> SidecarStatus {
        self.status_rx.borrow().clone()
    }
}

impl Drop for SidecarSupervisor {
    fn drop(&mut self) {
        self.kill();
    }
}
