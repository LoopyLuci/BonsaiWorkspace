//! Real-time progress streaming for long-running generation jobs.
//!
//! [`ProgressStreamer`] wraps a tokio mpsc channel so generation tools can
//! emit structured [`ProgressEvent`] messages that the daemon forwards to
//! clients as JSON-RPC notifications.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

// ── Progress event ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProgressEvent {
    pub task_id: String,
    pub progress: f32,  // [0.0, 1.0]
    pub status: String, // "pending" | "running" | "completed" | "failed"
    pub message: String,
    pub timestamp: u64,
}

impl ProgressEvent {
    pub fn new(task_id: impl Into<String>, progress: f32, message: impl Into<String>) -> Self {
        let status = if progress >= 1.0 {
            "completed"
        } else {
            "running"
        };
        Self {
            task_id: task_id.into(),
            progress: progress.clamp(0.0, 1.0),
            status: status.into(),
            message: message.into(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    pub fn failed(task_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            progress: 0.0,
            status: "failed".into(),
            message: reason.into(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

// ── Streamer ──────────────────────────────────────────────────────────────────

pub struct ProgressStreamer {
    tx: mpsc::Sender<ProgressEvent>,
    task_id: String,
}

impl ProgressStreamer {
    /// Create a new streamer + receiver pair.  `capacity` is the channel buffer.
    pub fn new(
        task_id: impl Into<String>,
        capacity: usize,
    ) -> (Self, mpsc::Receiver<ProgressEvent>) {
        let (tx, rx) = mpsc::channel(capacity);
        (
            Self {
                tx,
                task_id: task_id.into(),
            },
            rx,
        )
    }

    /// Emit a progress event.  Returns `false` if the receiver has been dropped.
    pub async fn emit(&self, progress: f32, message: impl Into<String>) -> bool {
        self.tx
            .send(ProgressEvent::new(&self.task_id, progress, message))
            .await
            .is_ok()
    }

    /// Emit a failure event.
    pub async fn fail(&self, reason: impl Into<String>) -> bool {
        self.tx
            .send(ProgressEvent::failed(&self.task_id, reason))
            .await
            .is_ok()
    }

    pub fn task_id(&self) -> &str {
        &self.task_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_event_status_and_clamping() {
        let running = ProgressEvent::new("task-1", 0.5, "halfway");
        assert_eq!(running.status, "running");
        assert_eq!(running.progress, 0.5);

        let completed = ProgressEvent::new("task-1", 1.0, "done");
        assert_eq!(completed.status, "completed");

        let clamped = ProgressEvent::new("task-1", 1.5, "overshoot");
        assert_eq!(clamped.progress, 1.0);
    }

    #[test]
    fn test_progress_event_failed() {
        let failed = ProgressEvent::failed("task-1", "oom");
        assert_eq!(failed.status, "failed");
        assert_eq!(failed.progress, 0.0);
        assert_eq!(failed.message, "oom");
    }

    #[tokio::test]
    async fn test_streamer_emits_events() {
        let (streamer, mut rx) = ProgressStreamer::new("task-1", 4);
        assert_eq!(streamer.task_id(), "task-1");

        assert!(streamer.emit(0.3, "working").await);
        let event = rx.recv().await.unwrap();
        assert_eq!(event.progress, 0.3);
        assert_eq!(event.task_id, "task-1");

        assert!(streamer.fail("crashed").await);
        let event = rx.recv().await.unwrap();
        assert_eq!(event.status, "failed");
    }

    #[tokio::test]
    async fn test_streamer_emit_fails_after_receiver_dropped() {
        let (streamer, rx) = ProgressStreamer::new("task-1", 4);
        drop(rx);
        assert!(!streamer.emit(0.5, "still going").await);
    }
}
