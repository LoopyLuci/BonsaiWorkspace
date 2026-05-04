use std::sync::Arc;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

use crate::platforms::{InboundMessage, MessagingPlatform};

// ── Scheduled task config ─────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScheduledTask {
    pub id:           String,
    pub name:         String,
    /// Target platform name ("discord", "telegram", "email", "matrix", "system")
    pub platform:     String,
    /// channel / chat_id to deliver to (empty string → broadcast to all)
    pub platform_id:  String,
    /// user_id to attribute the synthetic message to
    pub user_id:      String,
    /// Text of the synthetic inbound message (e.g., "generate daily digest")
    pub message:      String,
    /// Run every N seconds
    pub interval_secs: u64,
    #[serde(default = "bool_true")]
    pub enabled:      bool,
}

fn bool_true() -> bool { true }

// ── Scheduler state ───────────────────────────────────────────────────────────

pub struct Scheduler {
    tasks: Vec<ScheduledTask>,
}

impl Scheduler {
    /// Load tasks from `scheduled_tasks.json` in the config directory.
    /// Returns an empty scheduler if the file is missing or unreadable.
    pub fn load() -> Self {
        let path = task_file_path();
        let tasks = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str::<Vec<ScheduledTask>>(&s).ok())
                .unwrap_or_default()
        } else {
            // Write a commented example file on first run
            let example = vec![ScheduledTask {
                id:            "daily-digest".into(),
                name:          "Daily Digest".into(),
                platform:      "system".into(),
                platform_id:   "".into(),
                user_id:       "scheduler".into(),
                message:       "generate a brief daily status summary".into(),
                interval_secs: 86_400,
                enabled:       false,
            }];
            if let Ok(json) = serde_json::to_string_pretty(&example) {
                let _ = std::fs::create_dir_all(path.parent().unwrap_or(&PathBuf::from(".")));
                let _ = std::fs::write(&path, json);
            }
            Vec::new()
        };
        Self { tasks }
    }

    /// Spawn one Tokio task per enabled scheduled task. Each fires `interval_secs`
    /// and injects a synthetic `InboundMessage` into the inbound queue.
    pub fn spawn_all(
        self,
        tx: mpsc::Sender<InboundMessage>,
        platforms: Arc<Vec<Arc<dyn MessagingPlatform>>>,
    ) {
        for task in self.tasks.into_iter().filter(|t| t.enabled && t.interval_secs > 0) {
            let tx2       = tx.clone();
            let platforms2 = platforms.clone();
            let task2     = task.clone();

            tokio::spawn(async move {
                // Stagger startup: wait one full interval before the first fire
                tokio::time::sleep(Duration::from_secs(task2.interval_secs)).await;
                let mut ticker = interval(Duration::from_secs(task2.interval_secs));
                loop {
                    ticker.tick().await;

                    // If platform is "system" or empty, broadcast to the first available platform
                    let target_platform = if task2.platform.is_empty() || task2.platform == "system" {
                        platforms2.first().map(|p| p.name().to_string())
                    } else {
                        Some(task2.platform.clone())
                    };

                    let platform_name = match target_platform {
                        Some(n) => n,
                        None    => continue,
                    };

                    let inbound = InboundMessage {
                        platform:     platform_name,
                        platform_id:  task2.platform_id.clone(),
                        user_id:      task2.user_id.clone(),
                        display_name: format!("Scheduler:{}", task2.name),
                        event_id:     format!("sched:{}:{}", task2.id, chrono::Utc::now().timestamp()),
                        text:         task2.message.clone(),
                        reply_to:     None,
                    };

                    if tx2.try_send(inbound).is_err() {
                        tracing::warn!("[scheduler] Queue full; skipping task '{}'", task2.name);
                    } else {
                        tracing::info!("[scheduler] Fired task '{}'", task2.name);
                    }
                }
            });
        }
    }
}

fn task_file_path() -> PathBuf {
    crate::config::config_dir()
        .map(|d| d.join("bonsai").join("scheduled_tasks.json"))
        .unwrap_or_else(|| PathBuf::from("scheduled_tasks.json"))
}
