//! Scheduler — drives dream cycles on a time-based and idle-based schedule.
//!
//! Triggers:
//!   - 02:00 local time (full nightly consolidation)
//!   - Every `idle_trigger_mins` minutes if there are pending nodes and the user
//!     appears to be idle (no new nodes in the idle window)

use std::time::Duration;

use log::{info, warn};
use tokio::time::sleep;

use crate::config::Config;
use crate::dream_executor;
use crate::memory_nodes::MemoryNodeStore;

pub struct Scheduler {
    store: MemoryNodeStore,
    cfg: Config,
}

impl Scheduler {
    pub fn new(store: MemoryNodeStore, cfg: Config) -> Self {
        Self { store, cfg }
    }

    pub async fn run(self) {
        info!(
            "[scheduler] started — idle_trigger={}min",
            self.cfg.idle_trigger_mins
        );

        let idle_window = Duration::from_secs(self.cfg.idle_trigger_mins * 60);
        let mut last_nightly = None::<chrono::NaiveDate>;
        let poll_interval = Duration::from_secs(60);

        loop {
            sleep(poll_interval).await;

            let now = chrono::Local::now();
            let today = now.date_naive();

            // ── Nightly trigger at 02:00 ────────────────────────────────────
            if now.format("%H:%M").to_string().starts_with("02:0")
                && last_nightly.map_or(true, |d| d < today)
            {
                last_nightly = Some(today);
                info!(
                    "[scheduler] nightly consolidation triggered at {}",
                    now.format("%H:%M")
                );
                self.run_cycle("nightly").await;
                continue;
            }

            // ── Idle trigger ─────────────────────────────────────────────────
            let pending = self.store.pending_count().await.unwrap_or(0);
            if pending >= 50 {
                // Check if the last inserted node is older than the idle window
                if self.is_idle(idle_window).await {
                    info!("[scheduler] idle trigger: {pending} pending nodes");
                    self.run_cycle("idle").await;
                }
            }
        }
    }

    async fn run_cycle(&self, reason: &str) {
        let ws = self.cfg.workspace_path.as_deref();
        match dream_executor::run_dream_cycle(&self.store, &self.cfg, ws).await {
            Ok(summary) => {
                info!("[scheduler] cycle ({reason}) complete: {summary}");
                self.notify_app(&summary).await;
            }
            Err(e) => {
                warn!("[scheduler] cycle ({reason}) failed: {e}");
            }
        }
    }

    async fn is_idle(&self, window: Duration) -> bool {
        // Heuristic: if most-recent node timestamp is older than the idle window,
        // the user is likely idle.
        if let Ok(recent) = self.store.get_pending_nodes().await {
            if let Some(latest) = recent.last() {
                let age_ms = (chrono::Utc::now().timestamp_millis() - latest.timestamp_ms) as u64;
                return age_ms > window.as_millis() as u64;
            }
        }
        false
    }

    /// POST a dream-cycle-completed event to the main app's API.
    async fn notify_app(&self, summary: &str) {
        let client = reqwest::Client::new();
        let url = format!("{}/api/events/emit", self.cfg.app_api_url());
        let payload = serde_json::json!({
            "event": "dream-cycle-completed",
            "payload": { "summary": summary }
        });
        if let Err(e) = client
            .post(&url)
            .json(&payload)
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            // Non-fatal — app may not be running
            info!("[scheduler] could not notify app: {e}");
        }
    }
}
