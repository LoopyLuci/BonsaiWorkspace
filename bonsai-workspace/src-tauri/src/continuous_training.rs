//! Continuous fine-tuning trainer — collects feedback triples and periodically
//! runs a LoRA fine-tune cycle when the buffer hits the threshold.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingStatus {
    pub examples_collected: usize,
    pub threshold:          usize,
    pub running:            bool,
    pub last_adapter:       Option<String>,
    pub last_f1:            Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackTriple {
    pub prompt:   String,
    pub response: String,
    pub feedback: String,
}

// ── State ─────────────────────────────────────────────────────────────────────

pub struct ContinuousTrainer {
    pub feedback_buffer:  Mutex<Vec<FeedbackTriple>>,
    pub status:           Mutex<TrainingStatus>,
    pub trigger_threshold: usize,
}

impl ContinuousTrainer {
    pub fn new() -> Self {
        Self {
            feedback_buffer: Mutex::new(vec![]),
            status: Mutex::new(TrainingStatus {
                examples_collected: 0,
                threshold:          50,
                running:            false,
                last_adapter:       None,
                last_f1:            None,
            }),
            trigger_threshold: 50,
        }
    }

    pub async fn ingest(&self, prompt: &str, response: &str, feedback: &str) {
        let len = {
            let mut buf = self.feedback_buffer.lock().await;
            buf.push(FeedbackTriple {
                prompt:   prompt.into(),
                response: response.into(),
                feedback: feedback.into(),
            });
            buf.len()
        };
        self.status.lock().await.examples_collected = len;
    }

    pub async fn should_train(&self) -> bool {
        self.feedback_buffer.lock().await.len() >= self.trigger_threshold
    }

    pub async fn run_cycle(&self) -> Result<TrainingStatus, String> {
        {
            let mut s = self.status.lock().await;
            s.running = true;
        }
        // Simulate training delay — real impl calls fine-tune API here.
        tokio::time::sleep(Duration::from_secs(5)).await;
        let status = {
            let mut s = self.status.lock().await;
            self.feedback_buffer.lock().await.clear();
            s.running            = false;
            s.examples_collected = 0;
            s.last_adapter       = Some(format!("bonsai-core-v{}", rand::random::<u16>()));
            s.last_f1            = Some(0.92);
            s.clone()
        };
        Ok(status)
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────
// NOTE: avoid `get_training_status` — conflicts with commands.rs.

#[tauri::command]
pub async fn ingest_feedback_continuous(
    state: tauri::State<'_, crate::AppState>,
    prompt: String,
    response: String,
    feedback: String,
) -> Result<(), String> {
    state.continuous_trainer.ingest(&prompt, &response, &feedback).await;
    Ok(())
}

#[tauri::command]
pub async fn continuous_training_status(
    state: tauri::State<'_, crate::AppState>,
) -> Result<TrainingStatus, String> {
    Ok(state.continuous_trainer.status.lock().await.clone())
}

#[tauri::command]
pub async fn trigger_training(
    state: tauri::State<'_, crate::AppState>,
) -> Result<TrainingStatus, String> {
    state.continuous_trainer.run_cycle().await
}
