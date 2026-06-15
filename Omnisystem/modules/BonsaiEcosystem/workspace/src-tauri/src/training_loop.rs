//! Controlled continuous training loop.
//!
//! Runs two models concurrently on the same prompts, scores the gap,
//! synthesises optimal training examples from the reference's correct answers,
//! and queues a fine-tune run when enough examples accumulate.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex, RwLock};
use tracing::{info, warn};

use crate::dual_inference::{DualModelSession, SharedServer};
use crate::model_orchestrator::ModelOrchestrator;
use crate::telemetry::TelemetryStore;

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopConfig {
    /// Path to the base GGUF model that both BonsAI and reference use.
    pub base_model_path: String,
    /// Path to the BonsAI LoRA adapter.
    pub bonsai_adapter_path: String,
    /// Where to write synthesised JSONL training data.
    pub output_data_path: String,
    /// Where to write the next fine-tuned adapter.
    pub output_adapter_path: String,
    /// GPU layers for the shared llama-server (default 35 for 7900 XTX + 35B MoE).
    pub gpu_layers: u32,
    /// Kick off a fine-tune when this many new examples have been collected.
    pub finetune_threshold: usize,
    /// Prompts to rotate through; if empty the loop uses built-in seed prompts.
    pub prompts: Vec<String>,
    /// Seconds between comparison rounds (0 = as fast as possible).
    pub interval_secs: u64,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            base_model_path: String::new(),
            bonsai_adapter_path: String::new(),
            output_data_path: format!(
                "{}/.bonsai/data/loop_generated.jsonl",
                dirs::home_dir().unwrap_or_default().to_string_lossy()
            ),
            output_adapter_path: format!(
                "{}/.bonsai/adapters/bonsai-loop-latest",
                dirs::home_dir().unwrap_or_default().to_string_lossy()
            ),
            gpu_layers: 35,
            finetune_threshold: 50,
            prompts: Vec::new(),
            interval_secs: 0,
        }
    }
}

// ── Status ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Default)]
pub struct LoopStatus {
    pub running: bool,
    pub rounds_completed: u64,
    pub examples_collected: usize,
    pub last_intent_match_pct: f64,
    pub last_tool_overlap_pct: f64,
    pub finetune_queued: bool,
    pub last_error: Option<String>,
    pub elapsed_secs: u64,
}

// ── Training example ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct TrainingExample {
    /// Input prompt shown to both models.
    instruction: String,
    /// The reference model's output (ground truth).
    output: String,
    /// BonsAI's output (used to understand current gaps).
    bonsai_output: String,
    /// Intent extracted from reference output.
    intent: Option<String>,
    /// Gap types identified (e.g. "missing_tool", "intent_mismatch").
    gaps: Vec<String>,
}

// ── Built-in seed prompts ────────────────────────────────────────────────────

fn seed_prompts() -> Vec<String> {
    vec![
        // Coding
        "Create a Python script that reads a CSV file and outputs the top 5 rows.".into(),
        "List all files in the current directory that are larger than 1 MB.".into(),
        "Write a Rust function that computes the nth Fibonacci number iteratively.".into(),
        "Find all occurrences of 'TODO' in the current project source files.".into(),
        "Write a bash script to recursively find all .log files.".into(),
        "Create a new file called notes.txt with today's date as content.".into(),
        // System & tools
        "Run a system stats check and report CPU and memory usage.".into(),
        "Search the knowledge base for information about machine learning.".into(),
        // Music production
        "Generate a lo-fi hip hop beat at 85 bpm, 30 seconds.".into(),
        "Make an ambient dark drone in D minor, 45 seconds.".into(),
        "Create an upbeat major pentatonic melody at 120 bpm, 20 seconds.".into(),
        "Generate epic orchestral music with a slow tempo, 40 seconds.".into(),
        "Make a blues guitar-style track in A at 95 bpm, 25 seconds.".into(),
        "Compose a chill jazz chord progression at 70 bpm, 30 seconds.".into(),
        // General conversation & reasoning
        "Explain the difference between TCP and UDP in simple terms.".into(),
        "What are the SOLID principles in software engineering?".into(),
        "How does a transformer neural network work at a high level?".into(),
        "Describe the key differences between Rust and C++ memory management.".into(),
    ]
}

// ── Loop state ────────────────────────────────────────────────────────────────

pub struct TrainingLoop {
    orchestrator: Arc<ModelOrchestrator>,
    telemetry: Arc<TelemetryStore>,
    status: Arc<RwLock<LoopStatus>>,
    stop_flag: Arc<Mutex<bool>>,
    pub progress_tx: broadcast::Sender<serde_json::Value>,
}

impl TrainingLoop {
    pub fn new(orchestrator: Arc<ModelOrchestrator>, telemetry: Arc<TelemetryStore>) -> Self {
        let (progress_tx, _) = broadcast::channel(16);
        Self {
            orchestrator,
            telemetry,
            status: Arc::new(RwLock::new(LoopStatus::default())),
            stop_flag: Arc::new(Mutex::new(false)),
            progress_tx,
        }
    }

    pub async fn status(&self) -> LoopStatus {
        self.status.read().await.clone()
    }

    pub async fn stop(&self) {
        *self.stop_flag.lock().await = true;
    }

    pub async fn is_running(&self) -> bool {
        self.status.read().await.running
    }

    /// Spawn the loop as a background task. Returns immediately.
    pub async fn start(self: Arc<Self>, config: LoopConfig) -> Result<(), String> {
        if self.is_running().await {
            return Err("Training loop already running".into());
        }

        // Validate paths
        if config.base_model_path.is_empty() {
            return Err("base_model_path is required".into());
        }
        if config.bonsai_adapter_path.is_empty() {
            return Err("bonsai_adapter_path is required".into());
        }

        // Ensure output directories exist
        if let Some(parent) = Path::new(&config.output_data_path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create output dir: {e}"))?;
        }

        // Reset stop flag
        *self.stop_flag.lock().await = false;
        self.status.write().await.running = true;

        let this = self.clone();
        tokio::spawn(async move {
            if let Err(e) = this.run_loop(config).await {
                warn!(error = %e, "[training_loop] loop exited with error");
                let mut s = this.status.write().await;
                s.running = false;
                s.last_error = Some(e);
            }
        });

        Ok(())
    }

    async fn run_loop(&self, config: LoopConfig) -> Result<(), String> {
        let started_at = Instant::now();
        let prompts = if config.prompts.is_empty() {
            seed_prompts()
        } else {
            config.prompts.clone()
        };
        let mut prompt_idx = 0usize;
        let mut examples_this_run = 0usize;

        // Reuse the orchestrator's already-running llama-server slot.
        // Parse the port from its base URL (e.g. "http://127.0.0.1:8080").
        let slot_url = self
            .orchestrator
            .active_slot_url()
            .await
            .ok_or_else(|| "No active model slot — load a model first".to_string())?;
        let port: u16 = slot_url
            .trim_end_matches('/')
            .rsplit(':')
            .next()
            .and_then(|p| p.parse().ok())
            .ok_or_else(|| format!("Cannot parse port from slot URL: {slot_url}"))?;
        let server = Arc::new(SharedServer::from_existing_port(port));

        loop {
            // Check stop flag
            if *self.stop_flag.lock().await {
                info!("[training_loop] stop requested");
                break;
            }

            let prompt = &prompts[prompt_idx % prompts.len()];
            prompt_idx += 1;

            info!(round = prompt_idx, prompt = %prompt, "[training_loop] comparing models");

            let session = DualModelSession::new(server.clone(), None);
            let comparison = match session.compare(prompt).await {
                Ok(c) => c,
                Err(e) => {
                    warn!(error=%e, "[training_loop] comparison failed, continuing");
                    let mut s = self.status.write().await;
                    s.last_error = Some(e);
                    continue;
                }
            };

            // Update status
            {
                let mut s = self.status.write().await;
                s.rounds_completed += 1;
                s.last_intent_match_pct = if comparison.intent_match { 100.0 } else { 0.0 };
                s.last_tool_overlap_pct = comparison.tool_overlap_pct;
                s.elapsed_secs = started_at.elapsed().as_secs();
            }
            self.progress_tx
                .send(serde_json::json!({
                    "round": prompt_idx,
                    "gaps_found": comparison.gaps.len(),
                    "examples": examples_this_run,
                    "tool_overlap_pct": comparison.tool_overlap_pct,
                    "elapsed_secs": started_at.elapsed().as_secs()
                }))
                .ok();

            // Only write training data when there are gaps to learn from
            if !comparison.gaps.is_empty() || !comparison.intent_match {
                let example = TrainingExample {
                    instruction: prompt.clone(),
                    output: comparison.reference_tools.join(", ").to_string(),
                    bonsai_output: comparison.bonsai_tools.join(", "),
                    intent: None, // reference intent not available in ComparisonResult directly
                    gaps: comparison.gaps.iter().map(|g| g.gap_type.clone()).collect(),
                };

                if let Err(e) = self.append_example(&config.output_data_path, &example) {
                    warn!(error=%e, "[training_loop] failed to write example");
                } else {
                    examples_this_run += 1;
                    self.status.write().await.examples_collected = examples_this_run;
                }

                // Trigger fine-tune when threshold reached
                if examples_this_run >= config.finetune_threshold {
                    info!(
                        examples = examples_this_run,
                        "[training_loop] threshold reached, queuing fine-tune"
                    );
                    self.status.write().await.finetune_queued = true;
                    self.queue_finetune(&config).await;
                    examples_this_run = 0;
                    self.status.write().await.finetune_queued = false;
                }
            }

            // Pace the loop
            if config.interval_secs > 0 {
                tokio::time::sleep(Duration::from_secs(config.interval_secs)).await;
            }
        }

        self.status.write().await.running = false;
        Ok(())
    }

    fn append_example(&self, path: &str, example: &TrainingExample) -> Result<(), String> {
        use std::io::Write;
        let line = serde_json::to_string(example).map_err(|e| e.to_string())?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("Cannot open {path}: {e}"))?;
        writeln!(file, "{line}").map_err(|e| e.to_string())
    }

    async fn queue_finetune(&self, config: &LoopConfig) {
        // Reuse the existing trainer.rs interface via a subprocess.
        // This runs finetune.py asynchronously so the loop continues.
        let data = config.output_data_path.clone();
        let output = config.output_adapter_path.clone();
        tokio::spawn(async move {
            match crate::trainer::Trainer::run(None, &data, &output) {
                Ok(p) => info!(path=%p.display(), "[training_loop] fine-tune completed"),
                Err(e) => warn!(error=%e, "[training_loop] fine-tune failed"),
            }
        });
    }
}

// ── Shared loop state for AppState ───────────────────────────────────────────

pub struct TrainingLoopState {
    inner: Arc<TrainingLoop>,
}

impl TrainingLoopState {
    pub fn new(orchestrator: Arc<ModelOrchestrator>, telemetry: Arc<TelemetryStore>) -> Self {
        Self {
            inner: Arc::new(TrainingLoop::new(orchestrator, telemetry)),
        }
    }

    pub async fn start(&self, config: LoopConfig) -> Result<(), String> {
        self.inner.clone().start(config).await
    }

    pub async fn stop(&self) {
        self.inner.stop().await;
    }

    pub async fn status(&self) -> LoopStatus {
        self.inner.status().await
    }

    pub fn subscribe_progress(&self) -> broadcast::Receiver<serde_json::Value> {
        self.inner.progress_tx.subscribe()
    }
}
