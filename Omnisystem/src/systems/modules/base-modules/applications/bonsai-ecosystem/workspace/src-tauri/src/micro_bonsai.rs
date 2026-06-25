//! Micro BonsAI — intelligent model monitor and selector.
//!
//! An always-warm lightweight system (heuristic today, pluggable tiny GGUF
//! model tomorrow) that watches hardware resources and recent task context
//! to select the best model slot for each incoming request.
//!
//! Responsibilities:
//!  - Poll VRAM/RAM/CPU/GPU utilisation at 1 Hz.
//!  - Classify intent from the user prompt (maps to `TaskDomain`).
//!  - Score each available model against the classified intent + hardware.
//!  - Recommend a primary model, optional draft, and optional adapter.
//!  - Assemble custom `SwarmConfig` topologies on demand.
//!  - Expose perf history per `(task_type, model_id)` pair.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sysinfo::System;
use tokio::sync::{Mutex, RwLock};
use tokio::time::interval;

use crate::critic::TaskDomain;
use crate::model_registry::ModelRegistry;

// ── Constants ─────────────────────────────────────────────────────────────────

const TICK_HZ: u64 = 1;
const HISTORY_CAP: usize = 200;
const LOW_VRAM_THRESHOLD: u64 = 1024; // MiB — prefer smaller models below this
const LOW_RAM_THRESHOLD: u64 = 2048; // MiB

// ── Hardware snapshot ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HardwareSnapshot {
    pub vram_used_mb: u64,
    pub vram_total_mb: u64,
    pub ram_free_mb: u64,
    pub ram_total_mb: u64,
    pub cpu_utilisation_pct: f32,
    /// Rough GPU utilisation estimate (0–100). -1 means unavailable.
    pub gpu_utilisation_pct: f32,
    pub timestamp_secs: u64,
}

impl HardwareSnapshot {
    pub fn vram_free_mb(&self) -> u64 {
        self.vram_total_mb.saturating_sub(self.vram_used_mb)
    }
    pub fn ram_pressure(&self) -> f32 {
        if self.ram_total_mb == 0 {
            return 0.0;
        }
        1.0 - self.ram_free_mb as f32 / self.ram_total_mb as f32
    }
}

// ── Performance history ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfRecord {
    pub model_id: String,
    pub task_domain: String,
    pub tokens_per_sec: f32,
    pub quality_score: f32, // critic score 0–1, -1 if not available
    pub latency_ms: u32,
    pub succeeded: bool,
}

// ── Selection result ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelection {
    pub primary_model: String,
    pub draft_model: Option<String>,
    pub adapter: Option<String>,
    pub swarm_config_id: Option<String>,
    pub confidence: f32, // 0–1
    pub reasoning: String,
}

// ── Intent/context hint ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SelectionRequest {
    pub prompt: String,
    /// Tokens estimated in the prompt (rough).
    pub prompt_tokens: u32,
    /// Caller preference: speed vs quality 0=speed 1=quality.
    pub quality_bias: f32,
    /// Whether power-saving mode is on.
    pub power_saving: bool,
    /// Currently loaded model IDs across all slots.
    pub loaded_models: Vec<String>,
    /// All known model IDs the orchestrator can load.
    pub available_models: Vec<String>,
}

// ── Micro BonsAI ─────────────────────────────────────────────────────────────

pub struct MicroBonsai {
    hw: Arc<RwLock<HardwareSnapshot>>,
    history: Arc<Mutex<VecDeque<PerfRecord>>>,
    _ticker: tokio::task::JoinHandle<()>,
}

impl MicroBonsai {
    pub fn new() -> Arc<Self> {
        let hw = Arc::new(RwLock::new(HardwareSnapshot::default()));
        let history = Arc::new(Mutex::new(VecDeque::with_capacity(HISTORY_CAP)));

        let hw2 = Arc::clone(&hw);
        let ticker = tokio::spawn(async move {
            let mut tick = interval(Duration::from_secs(1000 / TICK_HZ / 1000 + 1));
            let mut sys = System::new_all();
            loop {
                tick.tick().await;
                sys.refresh_all();
                let ram_free = sys.available_memory() / 1024 / 1024;
                let ram_total = sys.total_memory() / 1024 / 1024;
                let cpu = sys.global_cpu_info().cpu_usage();

                let mut snap = hw2.write().await;
                snap.ram_free_mb = ram_free;
                snap.ram_total_mb = ram_total;
                snap.cpu_utilisation_pct = cpu;
                snap.timestamp_secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
            }
        });

        Arc::new(Self {
            hw,
            history,
            _ticker: ticker,
        })
    }

    // ── Hardware snapshot ─────────────────────────────────────────────────────

    pub async fn snapshot(&self) -> HardwareSnapshot {
        self.hw.read().await.clone()
    }

    /// Update VRAM figures (call after llama-server reports metrics).
    pub async fn update_vram(&self, used_mb: u64, total_mb: u64) {
        let mut snap = self.hw.write().await;
        snap.vram_used_mb = used_mb;
        snap.vram_total_mb = total_mb;
    }

    // ── Model selection ───────────────────────────────────────────────────────

    pub async fn select_model(&self, req: &SelectionRequest) -> ModelSelection {
        let hw = self.hw.read().await.clone();
        let hist = self.history.lock().await;
        let domain = TaskDomain::classify(&req.prompt);

        // Score each candidate model
        let candidates = if req.loaded_models.is_empty() {
            req.available_models.clone()
        } else {
            // Prefer already-loaded to avoid costly swap
            let mut v = req.loaded_models.clone();
            for m in &req.available_models {
                if !v.contains(m) {
                    v.push(m.clone());
                }
            }
            v
        };

        if candidates.is_empty() {
            return ModelSelection {
                primary_model: String::new(),
                draft_model: None,
                adapter: None,
                swarm_config_id: None,
                confidence: 0.0,
                reasoning: "No models available".into(),
            };
        }

        let mut best_id = candidates[0].clone();
        let mut best_score = f32::NEG_INFINITY;
        let mut reasoning_parts: Vec<String> = vec![];

        for model_id in &candidates {
            let score = self.score_model(model_id, &domain, &hw, req, &hist);
            if score > best_score {
                best_score = score;
                best_id = model_id.clone();
            }
        }

        // Determine domain-specific adapter hint
        let adapter = domain_adapter_hint(&domain);

        // Draft model: prefer a small model already loaded
        let draft = candidates
            .iter()
            .find(|m| *m != &best_id && is_small_model(m))
            .cloned();

        reasoning_parts.push(format!(
            "domain={:?} ram_free={}MiB vram_free={}MiB quality_bias={:.1}",
            domain,
            hw.ram_free_mb,
            hw.vram_free_mb(),
            req.quality_bias
        ));

        if req.power_saving {
            reasoning_parts.push("power-saving: preferred smallest model".into());
        }

        ModelSelection {
            primary_model: best_id,
            draft_model: draft,
            adapter,
            swarm_config_id: None,
            confidence: (best_score.tanh() * 0.5 + 0.5).clamp(0.0, 1.0),
            reasoning: reasoning_parts.join("; "),
        }
    }

    fn score_model(
        &self,
        model_id: &str,
        domain: &TaskDomain,
        hw: &HardwareSnapshot,
        req: &SelectionRequest,
        hist: &VecDeque<PerfRecord>,
    ) -> f32 {
        let mut score = 0.0f32;

        // Prefer loaded models (avoid cold-swap latency)
        if req.loaded_models.contains(&model_id.to_string()) {
            score += 2.0;
        }

        // Prefer domain-aligned models by name heuristic
        let id_lower = model_id.to_lowercase();
        match domain {
            TaskDomain::Code => {
                if id_lower.contains("code") || id_lower.contains("coder") {
                    score += 1.5;
                }
            }
            TaskDomain::Math => {
                if id_lower.contains("math") {
                    score += 1.5;
                }
            }
            TaskDomain::Creative => {
                if id_lower.contains("creative") || id_lower.contains("story") {
                    score += 1.0;
                }
            }
            _ => {}
        }

        // Penalise large models when resources are constrained
        if hw.ram_free_mb < LOW_RAM_THRESHOLD || hw.vram_free_mb() < LOW_VRAM_THRESHOLD {
            if is_large_model(model_id) {
                score -= 3.0;
            }
            if is_small_model(model_id) {
                score += 1.0;
            }
        }

        // Power saving: heavily prefer small models
        if req.power_saving && is_large_model(model_id) {
            score -= 5.0;
        }

        // Historical quality and speed
        let domain_str = format!("{:?}", domain);
        let relevant: Vec<&PerfRecord> = hist
            .iter()
            .filter(|r| r.model_id == model_id && r.task_domain == domain_str)
            .collect();
        if !relevant.is_empty() {
            let avg_q = relevant
                .iter()
                .map(|r| r.quality_score.max(0.0))
                .sum::<f32>()
                / relevant.len() as f32;
            let avg_tps =
                relevant.iter().map(|r| r.tokens_per_sec).sum::<f32>() / relevant.len() as f32;
            // blend quality and speed according to quality_bias
            score += avg_q * req.quality_bias * 2.0;
            score += (avg_tps / 50.0) * (1.0 - req.quality_bias);
        }

        // Bonsai native models get a small affinity bonus
        if id_lower.contains("bonsai") {
            score += 0.5;
        }

        score
    }

    // ── History recording ─────────────────────────────────────────────────────

    pub async fn record_perf(&self, record: PerfRecord) {
        let mut hist = self.history.lock().await;
        if hist.len() >= HISTORY_CAP {
            hist.pop_front();
        }
        hist.push_back(record);
    }

    // ── Swarm assembly ────────────────────────────────────────────────────────

    /// Suggest a swarm configuration ID for complex multi-step tasks.
    pub async fn suggest_swarm(&self, req: &SelectionRequest) -> Option<String> {
        let domain = TaskDomain::classify(&req.prompt);
        let hw = self.hw.read().await;

        // Only suggest a swarm if resources allow ≥ 2 model slots
        let can_swarm =
            hw.ram_free_mb >= LOW_RAM_THRESHOLD * 2 || hw.vram_free_mb() >= LOW_VRAM_THRESHOLD * 2;

        if !can_swarm {
            return None;
        }

        match domain {
            TaskDomain::Code => Some("dev-pipeline".into()),
            TaskDomain::Research => Some("parallel-then-synthesize".into()),
            _ => None,
        }
    }

    // ── Perf history export ───────────────────────────────────────────────────

    pub async fn perf_history(&self) -> Vec<PerfRecord> {
        self.history.lock().await.iter().cloned().collect()
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn is_small_model(id: &str) -> bool {
    let lower = id.to_lowercase();
    lower.contains("0.5b")
        || lower.contains("1b")
        || lower.contains("1.7b")
        || lower.contains("tiny")
        || lower.contains("mini")
}

fn is_large_model(id: &str) -> bool {
    let lower = id.to_lowercase();
    lower.contains("70b")
        || lower.contains("72b")
        || lower.contains("34b")
        || lower.contains("32b")
        || lower.contains("large")
}

fn domain_adapter_hint(domain: &TaskDomain) -> Option<String> {
    match domain {
        TaskDomain::Code => Some("code-lora".into()),
        TaskDomain::Math => Some("math-lora".into()),
        _ => None,
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct ModelSelectionRequest {
    pub prompt: String,
    pub quality_bias: Option<f32>,
    pub power_saving: Option<bool>,
}

#[tauri::command]
pub async fn micro_select_model(
    req: ModelSelectionRequest,
    state: tauri::State<'_, crate::AppState>,
) -> Result<ModelSelection, String> {
    let orchestrator = state.orchestrator.clone();
    let status = orchestrator.status().await;
    let all_models = orchestrator.list_models().await;

    let loaded: Vec<String> = status
        .slots
        .iter()
        .filter_map(|s| s.state.model_id().map(|m| m.to_string()))
        .collect();
    let available: Vec<String> = all_models.iter().map(|m| m.id.clone()).collect();

    let sel_req = SelectionRequest {
        prompt: req.prompt,
        prompt_tokens: 0,
        quality_bias: req.quality_bias.unwrap_or(0.7),
        power_saving: req.power_saving.unwrap_or(false),
        loaded_models: loaded,
        available_models: available,
    };

    let sel = state.micro_bonsai.select_model(&sel_req).await;
    Ok(sel)
}

#[tauri::command]
pub async fn micro_hardware_snapshot(
    state: tauri::State<'_, crate::AppState>,
) -> Result<HardwareSnapshot, String> {
    Ok(state.micro_bonsai.snapshot().await)
}

#[tauri::command]
pub async fn micro_perf_history(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<PerfRecord>, String> {
    Ok(state.micro_bonsai.perf_history().await)
}
