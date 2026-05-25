//! Self-play training loop: generate → critique → correct → curate.
//!
//! Uses the already-running ModelOrchestrator slot (no second server spawn).
//! Writes training examples in the same JSONL format as data_curator / finetune.py.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

use crate::model_orchestrator::ModelOrchestrator;
use crate::trainer::Trainer;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gap {
    pub seed_prompt: String,
    pub model_response: String,
    pub critique: String,
    pub corrected_response: String,
    /// Lexical novelty score (lower = more novel relative to existing data).
    pub overlap: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundResult {
    pub round_index: usize,
    pub gaps_found: usize,
    pub examples_added: usize,
    pub avg_overlap: f32,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfPlayConfig {
    /// Number of self-play rounds.
    pub rounds: usize,
    /// Temperature for exploratory (diverse) generation.
    pub temperature_high: f32,
    /// Temperature for critique and correction (focused/precise).
    pub temperature_low: f32,
    /// Overlap threshold — examples above this score are too similar to existing data and skipped.
    pub overlap_threshold: f32,
    /// Seeds to rotate through. Defaults to built-in domain seeds.
    pub seed_prompts: Vec<String>,
    /// Where to append curated JSONL examples.
    pub output_jsonl: PathBuf,
    /// Optional base model path for Trainer::run().
    pub base_model_path: Option<String>,
    /// Adapter output path.
    pub adapter_output: PathBuf,
    /// Trigger fine-tune after this many new examples.
    pub finetune_threshold: usize,
}

impl Default for SelfPlayConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        Self {
            rounds: 10,
            temperature_high: 0.9,
            temperature_low: 0.2,
            overlap_threshold: 0.85,
            seed_prompts: Vec::new(),
            output_jsonl: home.join(".bonsai/data/self_play.jsonl"),
            base_model_path: None,
            adapter_output: home.join(".bonsai/adapters/bonsai-self-play"),
            finetune_threshold: 50,
        }
    }
}

// ── SelfPlayStatus ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Default)]
pub struct SelfPlayStatus {
    pub running: bool,
    pub round: usize,
    pub total_rounds: usize,
    pub total_gaps: usize,
    pub total_examples: usize,
    pub avg_overlap: f32,
    pub last_error: Option<String>,
    pub elapsed_secs: u64,
}

// ── SelfPlayTrainer ────────────────────────────────────────────────────────────

pub struct SelfPlayTrainer {
    orchestrator: Arc<ModelOrchestrator>,
    config: SelfPlayConfig,
    status: Arc<RwLock<SelfPlayStatus>>,
    stop_flag: Arc<Mutex<bool>>,
}

impl SelfPlayTrainer {
    pub fn new(orchestrator: Arc<ModelOrchestrator>, config: SelfPlayConfig) -> Self {
        Self {
            orchestrator,
            config,
            status: Arc::new(RwLock::new(SelfPlayStatus::default())),
            stop_flag: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn status(&self) -> SelfPlayStatus {
        self.status.read().await.clone()
    }

    pub async fn stop(&self) {
        *self.stop_flag.lock().await = true;
    }

    /// Spawn the trainer as a background task.
    pub async fn start(self: Arc<Self>) -> Result<(), String> {
        if self.status.read().await.running {
            return Err("Self-play already running".into());
        }
        if let Some(parent) = self.config.output_jsonl.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create output dir: {e}"))?;
        }
        *self.stop_flag.lock().await = false;
        {
            let mut s = self.status.write().await;
            s.running = true;
            s.total_rounds = self.config.rounds;
        }
        let this = self.clone();
        tokio::spawn(async move {
            let status = this.status.clone();
            if let Err(e) = this.run().await {
                warn!(error=%e, "[self_play] exited with error");
                let mut s = status.write().await;
                s.running = false;
                s.last_error = Some(e);
            }
        });
        Ok(())
    }

    async fn run(self: Arc<Self>) -> Result<(), String> {
        let started = Instant::now();
        let seeds = if self.config.seed_prompts.is_empty() {
            default_seeds()
        } else {
            self.config.seed_prompts.clone()
        };

        let mut total_examples = 0usize;

        for i in 0..self.config.rounds {
            if *self.stop_flag.lock().await {
                info!("[self_play] stop requested");
                break;
            }

            info!(round=i, "[self_play] starting round");
            let result = match self.run_round(i, &seeds).await {
                Ok(r) => r,
                Err(e) => {
                    warn!(round=i, error=%e, "[self_play] round failed, continuing");
                    let mut s = self.status.write().await;
                    s.last_error = Some(e);
                    continue;
                }
            };

            total_examples += result.examples_added;
            {
                let mut s = self.status.write().await;
                s.round = i + 1;
                s.total_gaps += result.gaps_found;
                s.total_examples = total_examples;
                s.avg_overlap = result.avg_overlap;
                s.elapsed_secs = started.elapsed().as_secs();
            }

            if total_examples >= self.config.finetune_threshold {
                info!(examples=total_examples, "[self_play] threshold reached, triggering fine-tune");
                let data = self.config.output_jsonl.to_string_lossy().into_owned();
                let out = self.config.adapter_output.to_string_lossy().into_owned();
                let model = self.config.base_model_path.as_deref();
                match Trainer::run(model, &data, &out) {
                    Ok(p) => info!(path=%p.display(), "[self_play] fine-tune complete"),
                    Err(e) => warn!(error=%e, "[self_play] fine-tune failed"),
                }
                total_examples = 0;
            }
        }

        self.status.write().await.running = false;
        Ok(())
    }

    async fn run_round(&self, idx: usize, seeds: &[String]) -> Result<RoundResult, String> {
        let t0 = Instant::now();
        let mut gaps = Vec::new();

        for seed in seeds {
            if *self.stop_flag.lock().await { break; }

            // Step 1: generate an exploratory response
            let resp = self.infer(seed, self.config.temperature_high).await?;

            // Step 2: critique it
            let crit_prompt = format!(
                "You are a strict evaluator. Critique the following answer for accuracy, completeness, and usefulness. If it is good, say 'No issues'.\n\nQuestion: {seed}\nAnswer: {resp}\n\nCritique:"
            );
            let critique = self.infer(&crit_prompt, self.config.temperature_low).await?;

            // Skip if critique says it's fine
            if critique.to_lowercase().contains("no issues") || critique.len() < 20 {
                continue;
            }

            // Step 3: generate a corrected response
            let corr_prompt = format!(
                "Improve the following answer based on the critique.\n\nQuestion: {seed}\nOriginal answer: {resp}\nCritique: {critique}\n\nImproved answer:"
            );
            let corrected = self.infer(&corr_prompt, self.config.temperature_low).await?;

            // Step 4: novelty check (Jaccard word overlap against the original response)
            let overlap = word_overlap(&corrected, &resp);
            if overlap > self.config.overlap_threshold {
                continue; // correction too similar to original — skip
            }

            gaps.push(Gap {
                seed_prompt: seed.clone(),
                model_response: resp,
                critique,
                corrected_response: corrected,
                overlap,
            });
        }

        let n = gaps.len();
        let avg = if n > 0 {
            gaps.iter().map(|g| g.overlap).sum::<f32>() / n as f32
        } else {
            0.0
        };

        // Append accepted examples to JSONL
        if !gaps.is_empty() {
            self.append_examples(&gaps)
                .map_err(|e| format!("Failed to write examples: {e}"))?;
        }

        Ok(RoundResult {
            round_index: idx,
            gaps_found: n,
            examples_added: n,
            avg_overlap: avg,
            elapsed_ms: t0.elapsed().as_millis() as u64,
        })
    }

    async fn infer(&self, prompt: &str, _temperature: f32) -> Result<String, String> {
        // ModelOrchestrator::infer_simple doesn't expose temperature yet;
        // temperature is passed via the slot's sampling defaults.
        let (text, _stats) = self
            .orchestrator
            .infer_simple(prompt, 512, "self_play")
            .await?;
        Ok(text.trim().to_string())
    }

    fn append_examples(&self, gaps: &[Gap]) -> std::io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.output_jsonl)?;

        for gap in gaps {
            // Alpaca-style format expected by finetune.py
            let example = serde_json::json!({
                "text": format!(
                    "### Instruction:\n{}\n\n### Response:\n{}",
                    gap.seed_prompt, gap.corrected_response
                ),
                "source": "self_play",
                "confidence": 1.0 - gap.overlap,
            });
            writeln!(file, "{}", serde_json::to_string(&example).unwrap_or_default())?;
        }
        Ok(())
    }
}

// ── Shared state for AppState ─────────────────────────────────────────────────

pub struct SelfPlayState {
    inner: Arc<SelfPlayTrainer>,
}

impl SelfPlayState {
    pub fn new(orchestrator: Arc<ModelOrchestrator>, config: SelfPlayConfig) -> Self {
        Self {
            inner: Arc::new(SelfPlayTrainer::new(orchestrator, config)),
        }
    }

    pub async fn start(&self) -> Result<(), String> {
        self.inner.clone().start().await
    }

    pub async fn stop(&self) {
        self.inner.stop().await;
    }

    pub async fn status(&self) -> SelfPlayStatus {
        self.inner.status().await
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Word-level Jaccard similarity (proxy for overlap/novelty).
fn word_overlap(a: &str, b: &str) -> f32 {
    use std::collections::HashSet;
    let a: HashSet<&str> = a.split_whitespace().collect();
    let b: HashSet<&str> = b.split_whitespace().collect();
    let inter = a.intersection(&b).count();
    let union = a.union(&b).count();
    if union == 0 { 1.0 } else { inter as f32 / union as f32 }
}

fn default_seeds() -> Vec<String> {
    vec![
        "Create a Python script that reads a CSV and prints the top 5 rows.".into(),
        "Explain the difference between async/await and threading in Python.".into(),
        "Write a Rust function that parses a JSON string into a HashMap.".into(),
        "How would you design a rate limiter for an API endpoint?".into(),
        "What are the trade-offs between SQLite and PostgreSQL for a desktop app?".into(),
        "List files in the current directory larger than 1 MB.".into(),
        "Write a bash script to find all TODO comments in a source tree.".into(),
        "Explain how a transformer attention mechanism works.".into(),
        "What's the best strategy for fine-tuning a 1.7B language model on 500 examples?".into(),
        "Design a plugin system that is secure and extensible.".into(),
    ]
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn start_self_play(
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    state.self_play.start().await
}

#[tauri::command]
pub async fn stop_self_play(
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    state.self_play.stop().await;
    Ok(())
}

#[tauri::command]
pub async fn get_self_play_status(
    state: tauri::State<'_, crate::AppState>,
) -> Result<SelfPlayStatus, String> {
    Ok(state.self_play.status().await)
}
