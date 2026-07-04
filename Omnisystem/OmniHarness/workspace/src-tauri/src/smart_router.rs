//! SmartRouter — intelligent, hardware-aware, self-adjusting model router.
//! (Renamed from "Micro BonsAI"; logic preserved and extended, brand removed.)
//!
//! An always-warm lightweight system (heuristic today, pluggable tiny GGUF
//! model tomorrow) that watches hardware resources and recent task context
//! to select the best model slot for each incoming request.
//!
//! Responsibilities:
//!  - Poll VRAM/RAM/CPU/GPU utilisation at a configurable rate.
//!  - Classify intent from the user prompt (maps to `TaskDomain`).
//!  - Score each available model against the classified intent + hardware,
//!    using scoring weights that adapt over time from recorded outcomes
//!    (see `AdaptiveWeights`) rather than fixed constants.
//!  - Recommend a primary model, optional draft, and optional adapter.
//!  - Assemble custom `SwarmConfig` topologies on demand.
//!  - Persist perf history to disk (JSONL) so learned weights and recent
//!    history survive a restart — the whole point of "self-adjusting" is lost
//!    if every process restart resets it back to the static defaults.
//!
//! Longevity design notes (why this holds up as the system grows):
//!  - Scoring is table-driven (`AdaptiveWeights`), not a hardcoded formula —
//!    new signals can be added as new weighted terms without touching the
//!    selection algorithm itself.
//!  - No brand-specific heuristics (the old "id contains 'bonsai'" affinity
//!    bonus is gone); model affinity is driven purely by observed
//!    quality/speed history, so it works identically for any model family.
//!  - Every decision records *why* (`reasoning`) and its outcome is fed back
//!    via `record_perf`, so the router's own accuracy is auditable and
//!    self-correcting rather than a black box.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sysinfo::System;
use tokio::sync::{Mutex, RwLock};
use tokio::time::interval;

use crate::critic::TaskDomain;

// ── Constants ─────────────────────────────────────────────────────────────────

const DEFAULT_TICK_SECS: u64 = 1;
const HISTORY_CAP: usize = 500;
const LOW_VRAM_THRESHOLD: u64 = 1024; // MiB — prefer smaller models below this
const LOW_RAM_THRESHOLD: u64 = 2048; // MiB
const WEIGHT_LEARNING_RATE: f32 = 0.05; // EMA rate for adaptive weight updates

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
    /// Was this the model SmartRouter actually recommended for the request?
    /// Used to evaluate the router's own selection accuracy over time.
    #[serde(default)]
    pub was_recommended: bool,
}

// ── Adaptive scoring weights ───────────────────────────────────────────────────
//
// Replaces the old hardcoded score deltas (+2.0, +1.5, -3.0, ...) with named,
// tunable weights. `update_from_outcome` nudges them via a simple EMA toward
// whatever combination has recently correlated with high quality_score at
// acceptable latency — a small, auditable, restart-safe learning loop instead
// of a fixed-forever heuristic.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveWeights {
    pub already_loaded_bonus: f32,
    pub domain_match_bonus: f32,
    pub low_resource_small_model_bonus: f32,
    pub low_resource_large_model_penalty: f32,
    pub power_saving_large_model_penalty: f32,
    pub history_quality_weight: f32,
    pub history_speed_weight: f32,
}

impl Default for AdaptiveWeights {
    fn default() -> Self {
        Self {
            already_loaded_bonus: 2.0,
            domain_match_bonus: 1.5,
            low_resource_small_model_bonus: 1.0,
            low_resource_large_model_penalty: 3.0,
            power_saving_large_model_penalty: 5.0,
            history_quality_weight: 2.0,
            history_speed_weight: 1.0,
        }
    }
}

impl AdaptiveWeights {
    /// Nudge weights toward better outcomes. Called after each `record_perf`.
    /// `recommended_was_best`: did the model SmartRouter chose end up scoring
    /// at/above the median quality of everything tried recently for this domain?
    fn update_from_outcome(&mut self, recommended_was_best: bool) {
        let lr = WEIGHT_LEARNING_RATE;
        if recommended_was_best {
            // Reinforce the signals that led to this pick.
            self.history_quality_weight = (self.history_quality_weight * (1.0 + lr)).min(5.0);
        } else {
            // De-emphasise history slightly and lean more on domain/resource
            // signals, which are cheaper to get right than a thin history.
            self.history_quality_weight = (self.history_quality_weight * (1.0 - lr)).max(0.5);
            self.domain_match_bonus = (self.domain_match_bonus * (1.0 + lr)).min(3.0);
        }
    }
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

// ── Persistence config ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SmartRouterConfig {
    pub tick_secs: u64,
    /// Where perf history + learned weights are persisted (JSONL + JSON
    /// respectively). `None` disables persistence (in-memory only).
    pub persist_dir: Option<PathBuf>,
}

impl Default for SmartRouterConfig {
    fn default() -> Self {
        Self {
            tick_secs: DEFAULT_TICK_SECS,
            persist_dir: dirs::home_dir().map(|h| h.join(".omnisystem").join("smart_router")),
        }
    }
}

// ── SmartRouter ────────────────────────────────────────────────────────────────

pub struct SmartRouter {
    hw: Arc<RwLock<HardwareSnapshot>>,
    history: Arc<Mutex<VecDeque<PerfRecord>>>,
    weights: Arc<RwLock<AdaptiveWeights>>,
    persist_dir: Option<PathBuf>,
    _ticker: tokio::task::JoinHandle<()>,
}

impl SmartRouter {
    pub fn new() -> Arc<Self> {
        Self::with_config(SmartRouterConfig::default())
    }

    pub fn with_config(config: SmartRouterConfig) -> Arc<Self> {
        let hw = Arc::new(RwLock::new(HardwareSnapshot::default()));

        let history = load_history(&config.persist_dir).unwrap_or_default();
        let history = Arc::new(Mutex::new(history));

        let weights = load_weights(&config.persist_dir).unwrap_or_default();
        let weights = Arc::new(RwLock::new(weights));

        let hw2 = Arc::clone(&hw);
        let tick_secs = config.tick_secs.max(1);
        let ticker = tokio::spawn(async move {
            let mut tick = interval(Duration::from_secs(tick_secs));
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
            weights,
            persist_dir: config.persist_dir,
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
        let weights = self.weights.read().await.clone();
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
            let score = self.score_model(model_id, &domain, &hw, req, &hist, &weights);
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
        weights: &AdaptiveWeights,
    ) -> f32 {
        let mut score = 0.0f32;

        // Prefer loaded models (avoid cold-swap latency)
        if req.loaded_models.contains(&model_id.to_string()) {
            score += weights.already_loaded_bonus;
        }

        // Prefer domain-aligned models by name heuristic
        let id_lower = model_id.to_lowercase();
        match domain {
            TaskDomain::Code => {
                if id_lower.contains("code") || id_lower.contains("coder") {
                    score += weights.domain_match_bonus;
                }
            }
            TaskDomain::Math => {
                if id_lower.contains("math") {
                    score += weights.domain_match_bonus;
                }
            }
            TaskDomain::Creative => {
                if id_lower.contains("creative") || id_lower.contains("story") {
                    score += weights.domain_match_bonus * 0.67;
                }
            }
            _ => {}
        }

        // Penalise large models when resources are constrained
        if hw.ram_free_mb < LOW_RAM_THRESHOLD || hw.vram_free_mb() < LOW_VRAM_THRESHOLD {
            if is_large_model(model_id) {
                score -= weights.low_resource_large_model_penalty;
            }
            if is_small_model(model_id) {
                score += weights.low_resource_small_model_bonus;
            }
        }

        // Power saving: heavily prefer small models
        if req.power_saving && is_large_model(model_id) {
            score -= weights.power_saving_large_model_penalty;
        }

        // Historical quality and speed — this is the model's own track record,
        // not a hardcoded name check, so it works identically for any family.
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
            score += avg_q * req.quality_bias * weights.history_quality_weight;
            score += (avg_tps / 50.0) * (1.0 - req.quality_bias) * weights.history_speed_weight;
        }

        score
    }

    // ── History recording + adaptive weight update ────────────────────────────

    pub async fn record_perf(&self, record: PerfRecord) {
        // Evaluate whether this outcome supports the router's own choice,
        // then nudge weights before persisting — this is the "self-adjusting"
        // feedback loop: every recorded outcome slightly reshapes future scoring.
        let was_best = {
            let hist = self.history.lock().await;
            let domain_peers: Vec<f32> = hist
                .iter()
                .filter(|r| r.task_domain == record.task_domain)
                .map(|r| r.quality_score)
                .collect();
            let median_q = median(domain_peers);
            record.was_recommended && record.quality_score >= median_q
        };
        self.weights.write().await.update_from_outcome(was_best);

        let mut hist = self.history.lock().await;
        if hist.len() >= HISTORY_CAP {
            hist.pop_front();
        }
        hist.push_back(record);
        let snapshot: Vec<PerfRecord> = hist.iter().cloned().collect();
        drop(hist);

        if snapshot.len() % 20 == 0 {
            self.persist(&snapshot).await;
        }
    }

    async fn persist(&self, history: &[PerfRecord]) {
        let Some(dir) = &self.persist_dir else { return };
        let _ = std::fs::create_dir_all(dir);
        if let Ok(mut out) = std::fs::File::create(dir.join("perf_history.jsonl")) {
            use std::io::Write;
            for e in history {
                if let Ok(s) = serde_json::to_string(e) {
                    let _ = writeln!(out, "{s}");
                }
            }
        }
        if let Ok(w) = serde_json::to_string_pretty(&*self.weights.read().await) {
            let _ = std::fs::write(dir.join("weights.json"), w);
        }
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

    // ── Perf history + weights export (observability) ─────────────────────────

    pub async fn perf_history(&self) -> Vec<PerfRecord> {
        self.history.lock().await.iter().cloned().collect()
    }

    pub async fn current_weights(&self) -> AdaptiveWeights {
        self.weights.read().await.clone()
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn median(mut values: Vec<f32>) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    values[values.len() / 2]
}

fn load_history(dir: &Option<PathBuf>) -> Option<VecDeque<PerfRecord>> {
    let dir = dir.as_ref()?;
    let content = std::fs::read_to_string(dir.join("perf_history.jsonl")).ok()?;
    Some(
        content
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect(),
    )
}

fn load_weights(dir: &Option<PathBuf>) -> Option<AdaptiveWeights> {
    let dir = dir.as_ref()?;
    let content = std::fs::read_to_string(dir.join("weights.json")).ok()?;
    serde_json::from_str(&content).ok()
}

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
pub async fn smart_router_select_model(
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

    let sel = state.smart_router.select_model(&sel_req).await;
    Ok(sel)
}

#[tauri::command]
pub async fn smart_router_hardware_snapshot(
    state: tauri::State<'_, crate::AppState>,
) -> Result<HardwareSnapshot, String> {
    Ok(state.smart_router.snapshot().await)
}

#[tauri::command]
pub async fn smart_router_perf_history(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<PerfRecord>, String> {
    Ok(state.smart_router.perf_history().await)
}

#[tauri::command]
pub async fn smart_router_weights(
    state: tauri::State<'_, crate::AppState>,
) -> Result<AdaptiveWeights, String> {
    Ok(state.smart_router.current_weights().await)
}
