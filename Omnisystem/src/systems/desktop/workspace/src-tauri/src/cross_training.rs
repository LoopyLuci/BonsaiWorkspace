//! Cross-training pipeline — passively collects interaction events from chat,
//! plugins, and tool calls, buffers them into training examples, and triggers
//! fine-tuning via Trainer::run() when the buffer is large enough.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::{info, warn};

use crate::trainer::Trainer;

// ── Event types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum InteractionEvent {
    /// A plugin produced output in a given context.
    PluginOutput {
        plugin_id: String,
        output: String,
        context: String,
    },
    /// A tool was invoked with args and produced a result.
    ToolUsage {
        tool_name: String,
        args: String,
        result: String,
    },
    /// A chat turn completed (user ↔ assistant).
    ChatMessage { user: String, assistant: String },
}

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossTrainingConfig {
    /// Path to append collected JSONL examples.
    pub output_jsonl: PathBuf,
    /// Optional base GGUF model path for fine-tuning.
    pub base_model_path: Option<String>,
    /// Adapter output directory.
    pub adapter_output: PathBuf,
    /// Flush + fine-tune check interval in seconds.
    pub flush_interval_secs: u64,
    /// Buffer at least this many examples before triggering fine-tune.
    pub finetune_threshold: usize,
}

impl Default for CrossTrainingConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        Self {
            output_jsonl: home.join(".bonsai/data/cross_training.jsonl"),
            base_model_path: None,
            adapter_output: home.join(".bonsai/adapters/bonsai-cross"),
            flush_interval_secs: 300,
            finetune_threshold: 50,
        }
    }
}

// ── Pipeline ──────────────────────────────────────────────────────────────────

pub struct CrossTrainingPipeline {
    config: CrossTrainingConfig,
    rx: mpsc::UnboundedReceiver<InteractionEvent>,
}

impl CrossTrainingPipeline {
    /// Create pipeline + return sender. Caller keeps `tx` in AppState.
    pub fn new(config: CrossTrainingConfig) -> (Self, mpsc::UnboundedSender<InteractionEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { config, rx }, tx)
    }

    /// Run forever — call via `tokio::spawn`.
    pub async fn start(mut self) {
        if let Some(parent) = self.config.output_jsonl.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let mut tick = interval(Duration::from_secs(self.config.flush_interval_secs));
        let mut buf: Vec<String> = Vec::new();
        let mut example_count = 0usize;

        loop {
            tokio::select! {
                Some(event) = self.rx.recv() => {
                    let text = Self::event_to_text(&event);
                    buf.push(text);

                    if buf.len() >= 50 {
                        example_count += Self::flush_to_disk(&self.config.output_jsonl, &buf);
                        buf.clear();
                    }
                }
                _ = tick.tick() => {
                    if !buf.is_empty() {
                        example_count += Self::flush_to_disk(&self.config.output_jsonl, &buf);
                        buf.clear();
                    }

                    if example_count >= self.config.finetune_threshold {
                        info!(examples=example_count, "[cross_training] threshold reached, fine-tuning");
                        Self::run_finetune(&self.config);
                        example_count = 0;
                    }
                }
            }
        }
    }

    fn event_to_text(event: &InteractionEvent) -> String {
        match event {
            InteractionEvent::PluginOutput {
                plugin_id,
                output,
                context,
            } => format!("### Plugin {plugin_id}\nContext: {context}\nOutput: {output}"),
            InteractionEvent::ToolUsage {
                tool_name,
                args,
                result,
            } => format!("### Tool {tool_name}\nArgs: {args}\nResult: {result}"),
            InteractionEvent::ChatMessage { user, assistant } => {
                format!("### Instruction:\n{user}\n\n### Response:\n{assistant}")
            }
        }
    }

    fn flush_to_disk(path: &PathBuf, buf: &[String]) -> usize {
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            Err(e) => {
                warn!(error=%e, "[cross_training] cannot open JSONL");
                0
            }
            Ok(mut f) => {
                let mut written = 0;
                for text in buf {
                    let example = serde_json::json!({
                        "text": text,
                        "source": "cross_training",
                        "confidence": 0.8,
                    });
                    if writeln!(f, "{}", serde_json::to_string(&example).unwrap_or_default())
                        .is_ok()
                    {
                        written += 1;
                    }
                }
                info!(written, "[cross_training] flushed examples to disk");
                written
            }
        }
    }

    fn run_finetune(config: &CrossTrainingConfig) {
        let data = config.output_jsonl.to_string_lossy().into_owned();
        let out = config.adapter_output.to_string_lossy().into_owned();
        let model = config.base_model_path.as_deref();
        match Trainer::run(model, &data, &out) {
            Ok(p) => info!(path=%p.display(), "[cross_training] fine-tune complete"),
            Err(e) => warn!(error=%e, "[cross_training] fine-tune failed"),
        }
    }
}

// ── Sender handle for AppState ────────────────────────────────────────────────

#[derive(Clone)]
pub struct CrossTrainingSender(pub mpsc::UnboundedSender<InteractionEvent>);

impl CrossTrainingSender {
    pub fn send(&self, event: InteractionEvent) {
        let _ = self.0.send(event);
    }
}
