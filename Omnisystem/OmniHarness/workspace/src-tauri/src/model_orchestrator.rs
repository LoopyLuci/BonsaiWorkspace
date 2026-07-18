//! Production-grade model orchestrator.
//!
//! Manages a pool of llama-server processes (slots), routes inference requests,
//! maintains a back-pressure queue, enforces memory limits, health-monitors
//! each slot, and applies LRU eviction when switching models.
//!
//! # Architecture
//!
//!  ┌────────────────── OrchestratorLoop (single tokio task) ──────────────────┐
//!  │  ┌─────────┐  ┌─────────┐        ┌──────────────────┐  ┌─────────────┐  │
//!  │  │ Slot 0  │  │ Slot 1  │  ...   │   RequestQueue   │  │  Registry   │  │
//!  │  │ (Ready) │  │ (Empty) │        │   (VecDeque)     │  │  (GGUF cat) │  │
//!  │  └─────────┘  └─────────┘        └──────────────────┘  └─────────────┘  │
//!  └──────────────────────────────────────────────────────────────────────────┘
//!        ↑ OrchestratorCmd channel (mpsc)
//!        ↑ SlotFreed notifications from inference tasks

use std::collections::{HashMap, VecDeque};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::StreamExt;
use rand::RngExt;
use reqwest::Client;
use serde::Serialize;
use serde_json::json;
use sysinfo::System;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::bootstrap;
use crate::gpu_placement::{self, PlacementPlan};
use crate::inference_mode::InferenceMode;
use crate::model_data::GpuProfile;
use crate::model_registry::{ModelInfo, ModelRegistry, Quant};
use crate::sidecar_supervisor::{SidecarConfig, SidecarStatus, SidecarSupervisor};

const MODEL_LOAD_POLL_INTERVAL_MS: u64 = 500;
/// Maximum number of concurrent `infer_simple` callers.  The slot queue handles
/// the actual serialisation; this limit prevents runaway callers from piling up
/// unbounded oneshot channels.
static INFER_SEMAPHORE: std::sync::OnceLock<tokio::sync::Semaphore> = std::sync::OnceLock::new();
fn infer_semaphore() -> &'static tokio::sync::Semaphore {
    INFER_SEMAPHORE.get_or_init(|| tokio::sync::Semaphore::new(8))
}
#[cfg(target_os = "android")]
const MODEL_LOAD_TIMEOUT_SECS: u64 = 420;
#[cfg(not(target_os = "android"))]
const MODEL_LOAD_TIMEOUT_SECS: u64 = 240;
const MODEL_LOAD_TIMEOUT_GRACE_SECS: u64 = 120;
const MODEL_LOAD_MAX_POLLS: u64 = (MODEL_LOAD_TIMEOUT_SECS * 1000) / MODEL_LOAD_POLL_INTERVAL_MS;
const MODEL_LOAD_GRACE_POLLS: u64 =
    (MODEL_LOAD_TIMEOUT_GRACE_SECS * 1000) / MODEL_LOAD_POLL_INTERVAL_MS;
const MAX_MODEL_LOAD_ATTEMPTS: u8 = 3; // full placement + reduced-layer fallback + CPU-only

// ── Slot state ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SlotState {
    Empty,
    Loading {
        model_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        load_pct: Option<u32>,
    },
    Ready {
        model_id: String,
    },
    Busy {
        model_id: String,
    },
    Crashed {
        model_id: String,
        error: String,
    },
}

impl SlotState {
    pub fn model_id(&self) -> Option<&str> {
        match self {
            Self::Loading { model_id, .. }
            | Self::Ready { model_id }
            | Self::Busy { model_id }
            | Self::Crashed { model_id, .. } => Some(model_id),
            Self::Empty => None,
        }
    }
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready { .. })
    }
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading { .. })
    }
}

// ── Slot ──────────────────────────────────────────────────────────────────────

struct Slot {
    index: usize,
    port: u16,
    base_url: String,
    state: SlotState,
    process: Option<std::process::Child>,
    supervisor: Option<SidecarSupervisor>,
    last_used: Instant,
    total_requests: u64,
    load_started: Option<Instant>,
    current_model: Option<ModelInfo>,
    load_attempt: u8,
    gpu_layers: u32,
    cpu_mode: bool,
    inference_mode: InferenceMode,
    fallback_note: Option<String>,
    /// Remaining rungs of the graduated GPU/CPU retry ladder for the
    /// current load, index 0 being the currently-active placement. On a
    /// GPU crash, `poll_loading_slots` pops the front and retries with the
    /// next rung instead of jumping straight to CPU-only.
    gpu_ladder: Vec<gpu_placement::PlacementPlan>,
}

impl Slot {
    fn new(index: usize) -> Self {
        // Find a free port by trying to bind; avoids port collision race conditions
        let port = loop {
            let candidate = rand::rng().random_range(30_000u16..50_000u16);
            if let Ok(listener) = TcpListener::bind(("127.0.0.1", candidate)) {
                // Port is free; drop listener to release the binding
                drop(listener);
                break candidate;
            }
            // Port is busy or can't bind; try next
        };
        Self {
            index,
            port,
            base_url: format!("http://127.0.0.1:{}", port),
            state: SlotState::Empty,
            process: None,
            supervisor: None,
            last_used: Instant::now(),
            total_requests: 0,
            load_started: None,
            current_model: None,
            load_attempt: 0,
            gpu_layers: 0,
            cpu_mode: false,
            inference_mode: InferenceMode::default(),
            fallback_note: None,
            gpu_ladder: Vec::new(),
        }
    }

    fn kill(&mut self) {
        if let Some(sup) = self.supervisor.take() {
            sup.kill();
        } else if let Some(mut child) = self.process.take() {
            let _ = child.kill();
        }
        self.process = None;
        self.state = SlotState::Empty;
        self.current_model = None;
        self.load_attempt = 0;
        self.gpu_layers = 0;
        self.cpu_mode = false;
        self.inference_mode = InferenceMode::default();
        self.fallback_note = None;
        self.gpu_ladder.clear();
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.kill();
    }
}

// ── Public status types ───────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct SlotStatus {
    pub index: usize,
    pub port: u16,
    pub state: SlotState,
    pub requests: u64,
    pub idle_secs: u64,
    pub load_elapsed_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_note: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub cpu_mode: bool,
    pub inference_mode: InferenceMode,
}

#[derive(Serialize, Clone)]
pub struct OrchestratorStatus {
    pub slots: Vec<SlotStatus>,
    pub queue_depth: usize,
    pub total_ram_mb: u64,
    pub free_ram_mb: u64,
}

// ── Token stats ───────────────────────────────────────────────────────────────

#[derive(Serialize, Clone, Default, Debug)]
pub struct InferStats {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub tokens_per_second: f32,
    pub time_to_first_token_ms: u64,
    pub total_time_ms: u64,
}

// ── Per-request inference overrides ──────────────────────────────────────────

/// Sampling parameters that override the orchestrator's built-in defaults.
/// Populated from `ModelData::inference` when available; all fields optional.
#[derive(Debug, Clone, Default)]
pub struct InferenceOverrides {
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub min_p: Option<f32>,
    pub repeat_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub stop_sequences: Vec<String>,
}

// ── Infer request ─────────────────────────────────────────────────────────────

pub struct InferRequest {
    /// Which model to use; None = any ready slot.
    pub model_id: Option<String>,
    /// Full OpenAI-format message history (system + user + assistant turns).
    pub messages: Vec<serde_json::Value>,
    pub max_tokens: u32,
    /// Per-model sampling overrides from `ModelData`. None = use defaults.
    pub overrides: Option<InferenceOverrides>,
    /// If Some, tokens are streamed here instead of via the app event bus.
    pub stream_tx: Option<mpsc::UnboundedSender<String>>,
    /// Optional cancellation flag set by the UI to stop active generation.
    pub cancel_flag: Option<Arc<AtomicBool>>,
    pub resp_tx: oneshot::Sender<Result<(String, InferStats), String>>,
    /// Request source tag for fairness scheduling ("workspace" | "assistant").
    pub source: &'static str,
}

// ── Internal command ──────────────────────────────────────────────────────────

enum Cmd {
    Infer(InferRequest),
    Load {
        model_id: String,
        resp_tx: oneshot::Sender<Result<(), String>>,
    },
    Unload(usize),
    Status {
        resp_tx: oneshot::Sender<OrchestratorStatus>,
    },
    SetInferenceMode {
        model_id: String,
        mode: InferenceMode,
    },
    GetInferenceMode {
        model_id: String,
        resp_tx: oneshot::Sender<Option<InferenceMode>>,
    },
    RefreshRegistry,
    SlotFreed(usize),
}

// ── Public handle ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ModelOrchestrator {
    cmd_tx: mpsc::UnboundedSender<Cmd>,
    registry: Arc<Mutex<ModelRegistry>>,
}

impl ModelOrchestrator {
    /// `extra_dirs` are additional model directories beyond the bootstrap path.
    pub fn new(app: AppHandle, extra_dirs: Vec<std::path::PathBuf>) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Cmd>();
        let models_dir = bootstrap::models_dir(&app);

        let mut all_dirs: Vec<std::path::PathBuf> = vec![models_dir];
        for d in extra_dirs {
            if !all_dirs.contains(&d) {
                all_dirs.push(d);
            }
        }
        let dir_refs: Vec<&std::path::Path> = all_dirs.iter().map(|p| p.as_path()).collect();
        let registry = Arc::new(Mutex::new(ModelRegistry::scan_dirs_recursive(&dir_refs)));

        let reg2 = registry.clone();
        let cmd_tx2 = cmd_tx.clone();
        tauri::async_runtime::spawn(async move {
            event_loop(cmd_rx, cmd_tx2, reg2, app).await;
        });

        Self { cmd_tx, registry }
    }

    /// Submit a streaming inference request.
    pub fn infer(&self, req: InferRequest) -> Result<(), String> {
        self.cmd_tx
            .send(Cmd::Infer(req))
            .map_err(|_| "orchestrator offline".into())
    }

    /// Load a model by ID (non-blocking; use the returned receiver to await readiness).
    pub fn load(&self, model_id: String) -> oneshot::Receiver<Result<(), String>> {
        let (tx, rx) = oneshot::channel();
        let _ = self.cmd_tx.send(Cmd::Load {
            model_id,
            resp_tx: tx,
        });
        rx
    }

    pub fn unload(&self, slot: usize) {
        let _ = self.cmd_tx.send(Cmd::Unload(slot));
    }

    pub fn refresh_registry(&self) {
        let _ = self.cmd_tx.send(Cmd::RefreshRegistry);
    }

    pub fn set_inference_mode(&self, model_id: String, mode: InferenceMode) {
        let _ = self.cmd_tx.send(Cmd::SetInferenceMode { model_id, mode });
    }

    pub async fn get_inference_mode(&self, model_id: String) -> Option<InferenceMode> {
        let (tx, rx) = oneshot::channel();
        let _ = self.cmd_tx.send(Cmd::GetInferenceMode {
            model_id,
            resp_tx: tx,
        });
        rx.await.ok().flatten()
    }

    pub async fn is_model_loaded(&self, model_id: &str) -> bool {
        let status = self.status().await;
        status.slots.iter().any(|s| {
            s.state.model_id() == Some(model_id)
                && !matches!(s.state, SlotState::Empty | SlotState::Crashed { .. })
        })
    }

    pub async fn unload_model(&self, model_id: &str) {
        let status = self.status().await;
        for slot in status
            .slots
            .into_iter()
            .filter(|s| s.state.model_id() == Some(model_id))
        {
            self.unload(slot.index);
        }
    }

    pub async fn status(&self) -> OrchestratorStatus {
        let (tx, rx) = oneshot::channel();
        let _ = self.cmd_tx.send(Cmd::Status { resp_tx: tx });
        rx.await.unwrap_or_else(|_| OrchestratorStatus {
            slots: vec![],
            queue_depth: 0,
            total_ram_mb: 0,
            free_ram_mb: 0,
        })
    }

    pub async fn list_models(&self) -> Vec<ModelInfo> {
        self.registry.lock().await.models.clone()
    }

    /// Returns the base URL of the first Ready slot (for API proxying).
    pub async fn active_slot_url(&self) -> Option<String> {
        let (tx, rx) = oneshot::channel();
        let _ = self.cmd_tx.send(Cmd::Status { resp_tx: tx });
        let status = rx.await.ok()?;
        let probe = Client::new();
        resolve_active_slot_url(&status, &probe).await
    }

    /// Best-effort readiness details for user-facing diagnostics.
    pub async fn readiness_hint(&self) -> Option<String> {
        let status = self.status().await;

        let mut loading = Vec::new();
        for slot in status.slots.iter() {
            if let SlotState::Loading { model_id, load_pct } = &slot.state {
                let pct = load_pct
                    .map(|p| format!("{p}%"))
                    .unwrap_or_else(|| "starting".to_string());
                loading.push(format!("slot {}: {} ({})", slot.index, model_id, pct));
            }
        }
        if !loading.is_empty() {
            return Some(format!("Still loading: {}.", loading.join(", ")));
        }

        let mut crashed = Vec::new();
        for slot in status.slots.iter() {
            if let SlotState::Crashed { model_id, error } = &slot.state {
                crashed.push(format!("slot {}: {} ({})", slot.index, model_id, error));
            }
        }
        if !crashed.is_empty() {
            return Some(format!("Crashed slots: {}.", crashed.join(", ")));
        }

        None
    }

    /// Convenience wrapper: submit a single user-turn prompt and await the full
    /// response text. Used by the model-data generator and similar internal tools.
    pub async fn infer_simple(
        &self,
        prompt: &str,
        max_tokens: u32,
        source: &'static str,
    ) -> Result<(String, InferStats), String> {
        let _permit = infer_semaphore()
            .acquire()
            .await
            .map_err(|_| "inference semaphore closed".to_string())?;
        use serde_json::json;
        let messages = vec![json!({ "role": "user", "content": prompt })];
        let (resp_tx, resp_rx) = oneshot::channel();
        let req = InferRequest {
            model_id: None,
            messages,
            max_tokens,
            overrides: None,
            stream_tx: None,
            cancel_flag: None,
            resp_tx,
            source,
        };
        self.infer(req)?;
        resp_rx
            .await
            .map_err(|_| "orchestrator dropped the response channel".to_string())?
    }
}

// ── Event loop ────────────────────────────────────────────────────────────────

async fn event_loop(
    mut rx: mpsc::UnboundedReceiver<Cmd>,
    cmd_tx: mpsc::UnboundedSender<Cmd>,
    registry: Arc<Mutex<ModelRegistry>>,
    app: AppHandle,
) {
    let n_slots = decide_slot_count();
    let mut slots: Vec<Slot> = (0..n_slots).map(Slot::new).collect();
    let mut queue: VecDeque<InferRequest> = VecDeque::new();
    let mut inference_modes: HashMap<String, InferenceMode> = HashMap::new();

    let client = Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .unwrap_or_default();

    // No eager model pre-load here: spawning a llama-server process holds its
    // full weights + KV cache resident in RAM for as long as the app runs,
    // even if the user never opens chat. `Cmd::Infer`/`Cmd::Load` already
    // lazy-load on first real use (see `maybe_start_load` below), so idle
    // sessions no longer pay for a model nobody asked for.

    loop {
        tokio::select! {
            cmd = rx.recv() => match cmd {
                None => break,
                Some(c) => handle_cmd(c, &mut slots, &mut queue, &mut inference_modes, &cmd_tx, &registry, &client, &app).await,
            },
            // Periodic: advance queue + poll Loading slots for readiness.
            // Skipped when the pool is fully idle (no slots doing anything,
            // nothing queued) — `build_status` queries system memory on every
            // call, so ticking this 4x/sec forever with nothing to report was
            // pure overhead for a status display nobody was looking at.
            _ = tokio::time::sleep(Duration::from_millis(250)) => {
                let pool_idle = queue.is_empty() && slots.iter().all(|s| s.state.is_empty());
                if !pool_idle {
                    poll_loading_slots(&mut slots, &client, &app).await;
                    drain_queue(&mut queue, &mut slots, &cmd_tx, &client, &app).await;
                    emit_status(&slots, &queue, &app);
                }
            }
        }
    }
}

async fn handle_cmd(
    cmd: Cmd,
    slots: &mut Vec<Slot>,
    queue: &mut VecDeque<InferRequest>,
    inference_modes: &mut HashMap<String, InferenceMode>,
    cmd_tx: &mpsc::UnboundedSender<Cmd>,
    registry: &Arc<Mutex<ModelRegistry>>,
    client: &Client,
    app: &AppHandle,
) {
    match cmd {
        Cmd::Infer(req) => {
            let mid = req.model_id.as_deref();
            if let Some(idx) = best_ready_slot(slots, mid) {
                dispatch(idx, req, slots, cmd_tx, client, app);
            } else {
                // No ready slot — if a suitable model isn't loading, start it.
                // This is the *only* place a model gets spawned on a request
                // with no explicit model_id (e.g. `infer_simple` callers) —
                // deliberately lazy, so nothing loads until real work needs it.
                match req.model_id.clone() {
                    Some(mid_owned) => {
                        maybe_start_load(mid_owned, slots, registry, inference_modes, app).await;
                    }
                    None if slots
                        .iter()
                        .all(|s| matches!(s.state, SlotState::Empty | SlotState::Crashed { .. })) =>
                    {
                        let last_id = crate::config::load_config(app)
                            .ok()
                            .and_then(|c| c.last_model_id);
                        let reg = registry.lock().await;
                        let info = last_id
                            .as_deref()
                            .and_then(|id| reg.models.iter().find(|m| m.id == id).cloned())
                            .or_else(|| reg.models.first().cloned());
                        if let Some(info) = info {
                            drop(reg);
                            let mode = inference_modes.get(&info.id).cloned().unwrap_or_default();
                            let idx = empty_or_evict(slots);
                            spawn_model(&mut slots[idx], &info, app, &mode).await;
                        }
                    }
                    None => {}
                }
                queue.push_back(req);
            }
        }

        Cmd::Load { model_id, resp_tx } => {
            // Already ready?
            if slots
                .iter()
                .any(|s| s.state.model_id() == Some(&model_id) && s.state.is_ready())
            {
                let _ = resp_tx.send(Ok(()));
                return;
            }
            // Already loading?
            if slots
                .iter()
                .any(|s| s.state.model_id() == Some(&model_id) && s.state.is_loading())
            {
                // Poll until ready in background
                let url = slots
                    .iter()
                    .find(|s| s.state.model_id() == Some(&model_id))
                    .map(|s| s.base_url.clone())
                    .unwrap_or_default();
                tauri::async_runtime::spawn(async move {
                    let _ = resp_tx.send(wait_for_model_health(url).await);
                });
                return;
            }

            // Find or evict a slot
            let reg = registry.lock().await;
            let info = reg.models.iter().find(|m| m.id == model_id).cloned();
            drop(reg);

            match info {
                None => {
                    let _ = resp_tx.send(Err(format!("model {model_id} not in registry")));
                }
                Some(info) => {
                    let idx = empty_or_evict(slots);
                    let mode = inference_modes.get(&model_id).cloned().unwrap_or_default();
                    spawn_model(&mut slots[idx], &info, app, &mode).await;
                    let url = slots[idx].base_url.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = resp_tx.send(wait_for_model_health(url).await);
                    });
                }
            }
        }

        Cmd::Unload(idx) => {
            if let Some(slot) = slots.get_mut(idx) {
                slot.kill();
            }
        }

        Cmd::Status { resp_tx } => {
            let _ = resp_tx.send(build_status(slots, queue));
        }

        Cmd::SetInferenceMode { model_id, mode } => {
            inference_modes.insert(model_id, mode);
        }

        Cmd::GetInferenceMode { model_id, resp_tx } => {
            let _ = resp_tx.send(inference_modes.get(&model_id).cloned());
        }

        Cmd::RefreshRegistry => {
            // Re-scan all known model directories.
            {
                let mut reg = registry.lock().await;
                reg.refresh();
            }
            let _ = app.emit("registry-updated", ());
            // Deliberately no "all slots idle -> auto-load first model" fallback
            // here: this handler fires on the very first startup registry scan
            // too, which used to eagerly spawn a llama-server process (holding
            // a model's full weights + KV cache resident) before the user ever
            // opened chat. `Cmd::Infer` already lazy-loads on first real request
            // via `maybe_start_load`, so idle sessions no longer pay for it.
        }

        Cmd::SlotFreed(idx) => {
            if let Some(slot) = slots.get_mut(idx) {
                if let SlotState::Busy { model_id } = &slot.state.clone() {
                    slot.state = SlotState::Ready {
                        model_id: model_id.clone(),
                    };
                }
            }
            drain_queue(queue, slots, cmd_tx, client, app).await;
        }
    }
}

async fn wait_for_model_health(url: String) -> Result<(), String> {
    let probe = Client::new();
    for _ in 0..MODEL_LOAD_MAX_POLLS {
        tokio::time::sleep(Duration::from_millis(MODEL_LOAD_POLL_INTERVAL_MS)).await;
        if probe_model_ready(&probe, &url).await {
            return Ok(());
        }
    }

    // Grace period: some models complete loading just after the primary timeout.
    for _ in 0..MODEL_LOAD_GRACE_POLLS {
        tokio::time::sleep(Duration::from_millis(MODEL_LOAD_POLL_INTERVAL_MS)).await;
        if probe_model_ready(&probe, &url).await {
            return Ok(());
        }
    }

    Err(format!(
        "model load timeout after {}s (+{}s grace)",
        MODEL_LOAD_TIMEOUT_SECS, MODEL_LOAD_TIMEOUT_GRACE_SECS
    ))
}

async fn resolve_active_slot_url(status: &OrchestratorStatus, probe: &Client) -> Option<String> {
    if let Some(url) = status
        .slots
        .iter()
        .find(|s| s.state.is_ready())
        .map(|s| format!("http://127.0.0.1:{}", s.port))
    {
        return Some(url);
    }

    // Startup race guard: a slot can be process-healthy for a brief window
    // before the orchestrator poll loop transitions Loading -> Ready.
    for slot in status
        .slots
        .iter()
        .filter(|s| matches!(s.state, SlotState::Loading { .. }))
    {
        let url = format!("http://127.0.0.1:{}", slot.port);
        if probe_model_ready(probe, &url).await {
            return Some(url);
        }
    }

    // Last-chance probe: if any non-crashed slot process is healthy, allow the
    // caller to proceed even before the next orchestrator status transition.
    for slot in status
        .slots
        .iter()
        .filter(|s| !matches!(s.state, SlotState::Empty | SlotState::Crashed { .. }))
    {
        let url = format!("http://127.0.0.1:{}", slot.port);
        if probe_model_ready(probe, &url).await {
            return Some(url);
        }
    }

    None
}

async fn probe_model_ready(client: &Client, base_url: &str) -> bool {
    let health_ok = client
        .get(format!("{}/health", base_url))
        .send()
        .await
        .is_ok_and(|r| r.status().is_success());
    if health_ok {
        return true;
    }

    client
        .get(format!("{}/v1/models", base_url))
        .send()
        .await
        .is_ok_and(|r| r.status().is_success())
}

// ── Slot management ───────────────────────────────────────────────────────────

/// Loads this model's persisted GPU profile (crash history, last
/// known-good layer count) — the per-model replacement for the previous
/// global `AppConfig.gpu_crash_fallback` latch, which forced *every* model
/// on *every* future launch to skip GPU after a single crash on any one
/// model.
async fn load_gpu_profile(app: &AppHandle, model_id: &str) -> GpuProfile {
    let Some(state) = app.try_state::<crate::AppState>() else {
        return GpuProfile::default();
    };
    state
        .model_data_store
        .find_by_registry_id(model_id)
        .await
        .ok()
        .flatten()
        .map(|d| d.gpu_profile)
        .unwrap_or_default()
}

/// Persists an updated GPU profile for this model only — crash history and
/// placement outcomes never affect any other model's decisions. Self-heals
/// a missing `model_data` row (e.g. a model loaded before registry sync
/// completed) by constructing a fresh one rather than silently dropping the
/// update.
async fn save_gpu_profile(app: &AppHandle, info: &ModelInfo, profile: GpuProfile) {
    let Some(state) = app.try_state::<crate::AppState>() else {
        return;
    };
    let mut data = match state.model_data_store.find_by_registry_id(&info.id).await {
        Ok(Some(existing)) => existing,
        Ok(None) => crate::model_data::ModelData::from_registry(info),
        Err(_) => return,
    };
    data.gpu_profile = profile;
    data.touch();
    let _ = state.model_data_store.save(&data).await;
}

/// Human-readable summary of a placement plan for the UI fallback banner —
/// `None` for a plain, unremarkable full-GPU (or full-CPU-because-no-GPU)
/// offload that needs no explanation.
fn placement_plan_note(plan: &PlacementPlan, quant: &Quant) -> Option<String> {
    match plan {
        PlacementPlan::FullGpu(_) => None,
        PlacementPlan::TensorOverride { ot_rules, .. } => Some(format!(
            "GPU hybrid: {} tensor pattern(s) of this {:?} model kept on CPU, rest offloaded to GPU",
            ot_rules.len(),
            quant
        )),
        PlacementPlan::CpuMoe { n_cpu_moe, .. } => Some(format!(
            "GPU hybrid: {n_cpu_moe} MoE expert layer(s) kept on CPU, dense layers offloaded to GPU"
        )),
        PlacementPlan::Combined {
            ot_rules,
            n_cpu_moe,
            ..
        } => Some(format!(
            "GPU hybrid: {} tensor pattern(s) + {n_cpu_moe} MoE layer(s) kept on CPU, rest offloaded to GPU",
            ot_rules.len()
        )),
        PlacementPlan::CpuOnly => Some(format!(
            "GPU disabled: {quant:?} quantization has no safe partial-GPU placement for this model — running on CPU"
        )),
    }
}

/// Applies an explicit `InferenceMode` override on top of the computed
/// placement plan — matches the pre-existing precedent that a user's
/// explicit `CpuOnly`/`GpuOnly`/`Hybrid{n}` choice always wins over the
/// automatic decision, even overriding a confirmed-unsafe finding for
/// `GpuOnly`/`Hybrid` (the same escape hatch `mode.gpu_layers()` provided
/// before this system existed).
fn apply_mode_override(plan: PlacementPlan, mode: &InferenceMode) -> PlacementPlan {
    match mode {
        InferenceMode::Auto => plan,
        InferenceMode::CpuOnly => PlacementPlan::FullGpu(0),
        InferenceMode::GpuOnly => match plan {
            PlacementPlan::FullGpu(n) => PlacementPlan::FullGpu(n.max(1)),
            PlacementPlan::TensorOverride { ot_rules, gpu_layers } => PlacementPlan::TensorOverride {
                ot_rules,
                gpu_layers: gpu_layers.max(1),
            },
            PlacementPlan::CpuMoe { n_cpu_moe, gpu_layers } => PlacementPlan::CpuMoe {
                n_cpu_moe,
                gpu_layers: gpu_layers.max(1),
            },
            PlacementPlan::Combined { ot_rules, n_cpu_moe, gpu_layers } => PlacementPlan::Combined {
                ot_rules,
                n_cpu_moe,
                gpu_layers: gpu_layers.max(1),
            },
            PlacementPlan::CpuOnly => PlacementPlan::FullGpu(1),
        },
        InferenceMode::Hybrid { gpu_layers } => match plan {
            PlacementPlan::FullGpu(_) => PlacementPlan::FullGpu(*gpu_layers),
            PlacementPlan::TensorOverride { ot_rules, .. } => PlacementPlan::TensorOverride {
                ot_rules,
                gpu_layers: *gpu_layers,
            },
            PlacementPlan::CpuMoe { n_cpu_moe, .. } => PlacementPlan::CpuMoe {
                n_cpu_moe,
                gpu_layers: *gpu_layers,
            },
            PlacementPlan::Combined { ot_rules, n_cpu_moe, .. } => PlacementPlan::Combined {
                ot_rules,
                n_cpu_moe,
                gpu_layers: *gpu_layers,
            },
            PlacementPlan::CpuOnly => PlacementPlan::FullGpu(*gpu_layers),
        },
    }
}

/// Graduated GPU/CPU fallback ladder: on a crash, steps down through a
/// reduced layer count before giving up entirely, instead of the previous
/// single retry that jumped straight from the full plan to CPU-only.
/// Gated by `mode.allows_cpu_fallback()` exactly as the single-retry
/// behavior was before (`GpuOnly` never falls back).
fn build_gpu_ladder(plan: PlacementPlan, mode: &InferenceMode) -> Vec<PlacementPlan> {
    let mut ladder = vec![plan.clone()];
    if !mode.allows_cpu_fallback() {
        return ladder;
    }
    let gpu_layers = match &plan {
        PlacementPlan::FullGpu(n) => *n,
        PlacementPlan::TensorOverride { gpu_layers, .. }
        | PlacementPlan::CpuMoe { gpu_layers, .. }
        | PlacementPlan::Combined { gpu_layers, .. } => *gpu_layers,
        PlacementPlan::CpuOnly => 0,
    };
    if gpu_layers > 1 {
        ladder.push(PlacementPlan::FullGpu(gpu_layers / 2));
    }
    if gpu_layers != 0 {
        ladder.push(PlacementPlan::FullGpu(0));
    }
    ladder
}

/// Resolves a placement plan into the concrete `llama-server` CLI values:
/// `(--n-gpu-layers, --override-tensor rules, --n-cpu-moe)`.
fn placement_plan_cli(plan: &PlacementPlan) -> (u32, Vec<String>, Option<u32>) {
    match plan {
        PlacementPlan::FullGpu(n) => (*n, vec![], None),
        PlacementPlan::TensorOverride { ot_rules, gpu_layers } => {
            (*gpu_layers, ot_rules.clone(), None)
        }
        PlacementPlan::CpuMoe { n_cpu_moe, gpu_layers } => (*gpu_layers, vec![], Some(*n_cpu_moe)),
        PlacementPlan::Combined {
            ot_rules,
            n_cpu_moe,
            gpu_layers,
        } => (*gpu_layers, ot_rules.clone(), Some(*n_cpu_moe)),
        PlacementPlan::CpuOnly => (0, vec![], None),
    }
}

async fn spawn_model(slot: &mut Slot, info: &ModelInfo, app: &AppHandle, mode: &InferenceMode) {
    let gpu_profile = load_gpu_profile(app, &info.id).await;

    let exe = bootstrap::llama_exe(app);
    let exe_name = exe
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();
    let has_gpu = exe_name.contains("vulkan") || has_discrete_gpu();

    let (mut plan, mut note) = if !has_gpu {
        (
            PlacementPlan::FullGpu(0),
            Some("GPU disabled: no compatible discrete GPU detected".to_string()),
        )
    } else {
        let preferred_layers = estimate_safe_gpu_layers(&info.path);
        let vram_mb = query_dedicated_vram_mb().unwrap_or(4096);

        // Real per-tensor placement: instead of forcing the whole model
        // CPU-only when it contains a confirmed-unsafe quant, pin just
        // those tensors to CPU via `-ot` and offload the rest normally.
        match crate::gguf::parse_gguf_tensor_info(&info.path) {
            Ok((metadata, tensors)) => {
                let moe = gpu_placement::detect_moe(&metadata, &tensors);
                let plan = gpu_placement::build_placement_plan(
                    &tensors,
                    moe.as_ref(),
                    vram_mb,
                    preferred_layers,
                );
                let note = placement_plan_note(&plan, &info.quant);
                (plan, note)
            }
            Err(e) => {
                // Can't read real tensor data (corrupt file, permissions,
                // truncated download) — degrade to the previous
                // label-based heuristic rather than failing outright.
                tracing::warn!(
                    model_id = %info.id,
                    error = %e,
                    "[model-load] tensor_info parse failed, falling back to quant-label heuristic"
                );
                if is_gpu_unsafe_quant(&info.quant) {
                    (
                        PlacementPlan::CpuOnly,
                        Some(format!(
                            "GPU disabled: {:?} quantization is known to crash this GPU's Vulkan backend — running on CPU",
                            info.quant
                        )),
                    )
                } else {
                    (PlacementPlan::FullGpu(preferred_layers), None)
                }
            }
        }
    };

    // A previous crash on THIS specific model (not any other model) already
    // found a working layer count — start there directly instead of
    // re-attempting a plan already known to be unstable for this model.
    if gpu_profile.crash_count > 0 {
        if let Some(last_safe) = gpu_profile.last_safe_gpu_layers {
            plan = PlacementPlan::FullGpu(last_safe);
            note = Some(format!(
                "GPU: using last known-good layer count ({last_safe}) after {} previous crash(es) on this model",
                gpu_profile.crash_count
            ));
        }
    }

    let plan = apply_mode_override(plan, mode);

    tracing::debug!(
        model_id = %info.id,
        quant = ?info.quant,
        plan = ?plan,
        crash_count = gpu_profile.crash_count,
        mode = ?mode,
        "[model-load] placement decision"
    );

    let ladder = build_gpu_ladder(plan, mode);
    spawn_from_ladder(slot, info, app, ladder, 1, mode.clone(), note).await;
}

/// Returns true for quantization formats verified to crash the Vulkan backend
/// natively (STATUS_STACK_BUFFER_OVERRUN, exit 0xC0000409) as soon as any
/// layers are offloaded to GPU — reproduced directly by running llama-server
/// standalone: `Bonsai-1.7B-IQ1_S.gguf` crashes immediately with
/// `--n-gpu-layers` > 0, while the identical model in `Q2_K` loads and runs on
/// GPU without issue. This is a llama.cpp/driver Vulkan-kernel bug specific to
/// these exotic low-bit formats, not a VRAM-capacity problem — no amount of
/// layer-count or headroom tuning avoids it, so these quants always run
/// CPU-only regardless of available VRAM. `Unknown` quants (including the
/// BitNet ternary TQ1_0/TQ2_0 formats, which this registry doesn't yet
/// classify) are treated the same way out of caution, since they're the same
/// quantization family as the confirmed-crashing IQ1_S/IQ1_M.
pub(crate) fn is_gpu_unsafe_quant(quant: &Quant) -> bool {
    matches!(quant, Quant::IQ1_S | Quant::IQ1_M | Quant::Unknown(_))
}

/// Estimate a safe `--n-gpu-layers` value from free memory headroom and model
/// file size, instead of a flat guess.
///
/// This used to unconditionally request 20 layers for *any* model whenever a
/// discrete GPU was present, with no regard for the model's actual size or
/// available VRAM — which is exactly what caused GPU memory-fault crashes
/// (STATUS_STACK_BUFFER_OVERRUN / ErrorOutOfDeviceMemory, see
/// `is_gpu_memory_crash`/`is_gpu_memory_log`) on anything larger than the
/// smallest models, permanently latching `gpu_crash_fallback` and forcing
/// CPU-only for every model thereafter. Mirrors the size-aware calculation
/// already proven safe in `GpuModelLoader::calculate_gpu_layers`, generalized
/// since models of different parameter counts (1.7B/4B/8B/...) have different
/// transformer depths rather than a fixed 40 layers.
pub(crate) fn estimate_safe_gpu_layers(model_path: &std::path::Path) -> u32 {
    let file_size_mb = std::fs::metadata(model_path)
        .map(|m| m.len() / (1024 * 1024))
        .unwrap_or(0);
    if file_size_mb == 0 {
        return 0;
    }

    // Real dedicated-VRAM query (falls back to a conservative fixed estimate if
    // unavailable). Using total *system* RAM as a stand-in for VRAM — the
    // previous approach — wildly overestimates on any machine where RAM
    // capacity exceeds VRAM capacity (e.g. 64 GB system RAM vs. an 8-24 GB
    // GPU), which is the common case, not the exception.
    let free_mb = query_dedicated_vram_mb().unwrap_or(4096);
    let headroom_mb: u64 = 2048; // KV cache + compute buffers + fragmentation margin
    let usable_mb = free_mb.saturating_sub(headroom_mb);
    if usable_mb == 0 {
        return 0;
    }

    // Approximate transformer depth by file size bucket — close enough across
    // the common 1.7B/4B/8B/13B llama-family range to stay conservative.
    let estimated_total_layers: u64 = if file_size_mb < 3_000 {
        24
    } else if file_size_mb < 6_000 {
        32
    } else if file_size_mb < 10_000 {
        40
    } else {
        60
    };
    let per_layer_mb = (file_size_mb / estimated_total_layers).max(1);
    // Cap well under the full depth: the fused/recurrent compute buffers on
    // some architectures (e.g. Gated Delta Net MoE) need extra headroom the
    // weight-only estimate misses (verified empirically elsewhere in this
    // codebase — see GpuModelLoader::calculate_gpu_layers).
    let safe_max = estimated_total_layers.saturating_sub(5);
    ((usable_mb / per_layer_mb).min(safe_max)) as u32
}

/// Query dedicated video memory (MB) via WMI/CIM `AdapterRAM`. Known to
/// under-report on some Windows versions for GPUs above ~4 GB due to a
/// long-standing 32-bit-field overflow in `Win32_VideoController.AdapterRAM`
/// — that's an acceptable failure mode here (leads to a smaller, still-safe
/// `--n-gpu-layers` estimate rather than an over-estimate that risks another
/// out-of-memory crash). Returns the largest value across all adapters
/// (prefers the discrete GPU over an integrated one reporting less/no VRAM).
#[cfg(target_os = "windows")]
fn query_dedicated_vram_mb() -> Option<u64> {
    let mut c = std::process::Command::new("powershell");
    c.args([
        "-NoProfile",
        "-Command",
        "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty AdapterRAM",
    ]);
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let out = c.output().ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.trim().parse::<u64>().ok())
        .max()
        .map(|bytes| bytes / (1024 * 1024))
        .filter(|&mb| mb > 0)
}

#[cfg(not(target_os = "windows"))]
fn query_dedicated_vram_mb() -> Option<u64> {
    None
}

/// Spawns the model using the front of `ladder` as the placement to attempt
/// now, storing the remainder in `slot.gpu_ladder` for `poll_loading_slots`
/// to step through on a crash. Replaces the previous `spawn_model_with_layers`
/// (flat `gpu_layers: u32`) — the whole point of the graduated ladder is that
/// a crash steps down through progressively more conservative *placements*
/// (which may include `-ot`/`-ncmoe` args, not just a layer count) rather
/// than jumping straight to CPU-only in one retry.
async fn spawn_from_ladder(
    slot: &mut Slot,
    info: &ModelInfo,
    app: &AppHandle,
    mut ladder: Vec<PlacementPlan>,
    attempt: u8,
    mode: InferenceMode,
    fallback_note: Option<String>,
) {
    slot.kill();
    let plan = if ladder.is_empty() {
        PlacementPlan::FullGpu(0)
    } else {
        ladder.remove(0)
    };
    let (gpu_layers, ot_rules, n_cpu_moe) = placement_plan_cli(&plan);

    slot.current_model = Some(info.clone());
    slot.load_attempt = attempt;
    slot.gpu_layers = gpu_layers;
    slot.cpu_mode = gpu_layers == 0 && ot_rules.is_empty() && n_cpu_moe.is_none();
    slot.inference_mode = mode;
    slot.fallback_note = fallback_note;
    slot.gpu_ladder = ladder;
    slot.state = SlotState::Loading {
        model_id: info.id.clone(),
        load_pct: None,
    };
    slot.load_started = Some(Instant::now());

    let exe = bootstrap::llama_exe(app);
    if !exe.exists() {
        slot.state = SlotState::Crashed {
            model_id: info.id.clone(),
            error: "llama-server binary not found — bootstrap required".into(),
        };
        return;
    }

    let dir = exe.parent().unwrap_or(&exe).to_path_buf();
    let port_str = slot.port.to_string();
    let ctx = info.context_length.clamp(512, 4096).to_string();
    let threads = thread_count().to_string();
    let gpu_layers_str = gpu_layers.to_string();
    // `-ot`/`--override-tensor` accepts comma-separated `<pattern>=<buffer>`
    // pairs within a single flag invocation (confirmed via `--help`).
    let ot_arg = if ot_rules.is_empty() {
        None
    } else {
        Some(ot_rules.join(","))
    };
    let n_cpu_moe_str = n_cpu_moe.map(|n| n.to_string());

    // Pipe stderr to a per-slot log file so crash reasons are diagnosable.
    let log_dir = app
        .path()
        .app_log_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let stderr_log = log_dir.join(format!("llama-slot-{}.log", slot.index));
    let stderr_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&stderr_log)
        .ok()
        .map(std::process::Stdio::from)
        .unwrap_or_else(std::process::Stdio::null);

    let mut cmd = std::process::Command::new(&exe);
    cmd.args([
        "--port",
        &port_str,
        "--host",
        "127.0.0.1",
        "--model",
        &info.path.to_string_lossy(),
        "--ctx-size",
        &ctx,
        "--threads",
        &threads,
        "--n-gpu-layers",
        &gpu_layers_str,
        // Limit parallel slots to 1: the auto value (4) inflates compute-buffer
        // allocations by 4× and can push AMD Vulkan over its contiguous-heap limit.
        "--parallel",
        "1",
        // Disable flash attention: for Gemma 4 on AMD Vulkan it triggers a 547 MB
        // single-allocation for FA compute buffers which ErrorOutOfDeviceMemory.
        "--flash-attn",
        "off",
        "--no-warmup",
    ]);

    if let Some(ref ot) = ot_arg {
        cmd.args(["--override-tensor", ot]);
        tracing::info!(
            model_id = %info.id,
            ot_arg = %ot,
            "[orchestrator] tensor-level GPU/CPU hybrid placement active"
        );
    }
    if let Some(ref ncmoe) = n_cpu_moe_str {
        cmd.args(["--n-cpu-moe", ncmoe]);
        tracing::info!(
            model_id = %info.id,
            n_cpu_moe = %ncmoe,
            "[orchestrator] MoE expert weights kept on CPU"
        );
    }

    // Speculative decoding: wire in draft model when configured and available.
    // Yields 1.5–2.5× token throughput with identical output quality.
    if let Ok(cfg) = crate::config::load_config(app) {
        if let Some(ref draft_path) = cfg.draft_model_path {
            if std::path::Path::new(draft_path).exists() {
                cmd.args([
                    "--model-draft",
                    draft_path,
                    "--draft-max",
                    "8",
                    "--draft-min",
                    "1",
                    "--draft-p-split",
                    "0.1",
                ]);
                tracing::info!(draft=%draft_path, "[orchestrator] speculative decoding enabled");
            }
        }
        // Vision: wire mmproj for LLaVA when configured
        if let Some(ref mmproj) = cfg.vision_mmproj_path {
            if std::path::Path::new(mmproj).exists() {
                cmd.args(["--mmproj", mmproj]);
                tracing::info!(mmproj=%mmproj, "[orchestrator] vision mmproj loaded");
            }
        }
    }

    cmd.current_dir(&dir)
        .stdout(std::process::Stdio::null())
        .stderr(stderr_file);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }

    match cmd.spawn() {
        Ok(child) => {
            let cfg = SidecarConfig {
                base_url: slot.base_url.clone(),
                health_path: "/health".into(),
                load_timeout: Duration::from_secs(
                    MODEL_LOAD_TIMEOUT_SECS + MODEL_LOAD_TIMEOUT_GRACE_SECS,
                ),
                poll_interval: Duration::from_millis(MODEL_LOAD_POLL_INTERVAL_MS),
                log_path: Some(stderr_log.clone()),
            };
            slot.supervisor = Some(SidecarSupervisor::start(child, cfg));
            slot.process = None;
        }
        Err(e) => {
            slot.state = SlotState::Crashed {
                model_id: info.id.clone(),
                error: e.to_string(),
            };
        }
    }
}

fn best_ready_slot(slots: &[Slot], model_id: Option<&str>) -> Option<usize> {
    // Prefer an exact model match
    if let Some(mid) = model_id {
        if let Some(i) = slots
            .iter()
            .position(|s| s.state.is_ready() && s.state.model_id() == Some(mid))
        {
            return Some(i);
        }
    }
    // Any ready slot
    slots.iter().position(|s| s.state.is_ready())
}

fn empty_or_evict(slots: &mut Vec<Slot>) -> usize {
    if let Some(i) = slots.iter().position(|s| s.state.is_empty()) {
        return i;
    }
    // LRU eviction among Ready/Crashed slots (not Busy or Loading)
    slots
        .iter()
        .enumerate()
        .filter(|(_, s)| matches!(s.state, SlotState::Ready { .. } | SlotState::Crashed { .. }))
        .min_by_key(|(_, s)| s.last_used)
        .map(|(i, _)| i)
        .unwrap_or(0)
}

async fn maybe_start_load(
    model_id: String,
    slots: &mut Vec<Slot>,
    registry: &Arc<Mutex<ModelRegistry>>,
    inference_modes: &HashMap<String, InferenceMode>,
    app: &AppHandle,
) {
    // Don't load if already loading/ready
    if slots.iter().any(|s| s.state.model_id() == Some(&model_id)) {
        return;
    }
    let reg = registry.lock().await;
    if let Some(info) = reg.models.iter().find(|m| m.id == model_id).cloned() {
        drop(reg);
        let idx = empty_or_evict(slots);
        let mode = inference_modes.get(&model_id).cloned().unwrap_or_default();
        spawn_model(&mut slots[idx], &info, app, &mode).await;
    }
}

// ── Health polling ────────────────────────────────────────────────────────────

async fn poll_loading_slots(slots: &mut Vec<Slot>, client: &Client, app: &AppHandle) {
    for slot in slots.iter_mut() {
        if let SlotState::Loading { model_id, .. } = &slot.state.clone() {
            // Check if the process has exited unexpectedly via supervisor or raw child
            let exited = if let Some(sup) = &slot.supervisor {
                sup.try_wait()
            } else if let Some(ref mut child) = slot.process {
                child.try_wait().ok().flatten()
            } else {
                None
            };
            if let Some(status) = exited {
                let log_dir = app
                    .path()
                    .app_log_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."));
                let log_path = log_dir.join(format!("llama-slot-{}.log", slot.index));
                let detail = std::fs::read_to_string(&log_path)
                    .ok()
                    .and_then(|s| {
                        s.lines()
                            .filter(|l| !l.trim().is_empty())
                            .last()
                            .map(|l| l.to_owned())
                    })
                    .unwrap_or_default();
                let code = status.code();

                if should_retry_cpu_fallback(slot, code, &detail) {
                    let exit_code = code.unwrap_or_default();
                    let next_gpu_layers = slot
                        .gpu_ladder
                        .first()
                        .map(|p| placement_plan_cli(p).0)
                        .unwrap_or(0);
                    let note = if next_gpu_layers == 0 {
                        format!("GPU unstable (exit code {exit_code:#010X}) — switching to CPU mode")
                    } else {
                        format!(
                            "GPU unstable (exit code {exit_code:#010X}) — stepping down to {next_gpu_layers} GPU layer(s)"
                        )
                    };
                    tracing::warn!(
                        slot=%slot.index,
                        exit_code=%format!("{exit_code:#010X}"),
                        next_gpu_layers,
                        "[orchestrator] GPU crash detected, stepping down the placement ladder"
                    );

                    // Per-model crash isolation: persist this model's own
                    // crash history, not a global flag that would force
                    // every other model to skip GPU too.
                    if let Some(info) = slot.current_model.clone() {
                        let mut profile = load_gpu_profile(app, &info.id).await;
                        profile.crash_count += 1;
                        profile.last_crash_exit_code = Some(exit_code as u32);
                        profile.last_crash_at = Some(chrono::Utc::now().timestamp_millis());
                        save_gpu_profile(app, &info, profile).await;
                    }

                    let _ = app.emit(
                        "model-load-fallback",
                        json!({
                            "slot": slot.index,
                            "model_id": model_id,
                            "message": note,
                            "exit_code": format!("{exit_code:#010X}"),
                        }),
                    );

                    if let Some(info) = slot.current_model.clone() {
                        let next_attempt = (slot.load_attempt + 1).min(MAX_MODEL_LOAD_ATTEMPTS);
                        let remaining_ladder = std::mem::take(&mut slot.gpu_ladder);
                        spawn_from_ladder(
                            slot,
                            &info,
                            app,
                            remaining_ladder,
                            next_attempt,
                            slot.inference_mode.clone(),
                            Some(note),
                        )
                        .await;
                        continue;
                    }
                }

                let error = classify_model_load_error(code, &detail, status.to_string());
                slot.state = SlotState::Crashed {
                    model_id: model_id.clone(),
                    error,
                };
                slot.load_started = None;
                continue;
            }

            // Parse load progress from log: "llama_model_load: loaded N of M tensors (P%)"
            let log_dir = app
                .path()
                .app_log_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let log_path = log_dir.join(format!("llama-slot-{}.log", slot.index));
            if let Ok(log) = std::fs::read_to_string(&log_path) {
                // Walk lines in reverse to find the latest progress line
                let pct = log.lines().rev().find_map(|line| {
                    // Match "llama_model_load: loaded N of M tensors (P%)"
                    // or the briefer "loaded N of M tensors (P%)"
                    let paren = line.find('(')?;
                    let after = &line[paren + 1..];
                    let end = after.find('%')?;
                    after[..end].trim().parse::<u32>().ok()
                });
                if let Some(p) = pct {
                    if let SlotState::Loading { load_pct, .. } = &mut slot.state {
                        *load_pct = Some(p);
                    }
                    let _ = app.emit("model-load-progress", json!({
                        "slot":     slot.index,
                        "model_id": model_id,
                        "pct":      p,
                        "elapsed_secs": slot.load_started.map(|t| t.elapsed().as_secs()).unwrap_or(0),
                        "cpu_mode": slot.cpu_mode,
                        "fallback_note": slot.fallback_note,
                    }));
                }
            }

            // Check readiness via supervisor watch channel; fall back to direct HTTP probe.
            let ok = if let Some(sup) = &slot.supervisor {
                matches!(sup.status(), SidecarStatus::Ready)
            } else {
                probe_model_ready(client, &slot.base_url).await
            };
            if ok {
                slot.state = SlotState::Ready {
                    model_id: model_id.clone(),
                };
                slot.load_started = None;
                // Persist last-used model id so next startup can pre-load it
                if let Ok(mut cfg) = crate::config::load_config(app) {
                    cfg.last_model_id = Some(model_id.clone());
                    let _ = crate::config::save_config(app, &cfg);
                }
                // Remember the layer count that actually worked for THIS
                // model, so the next launch starts there directly instead
                // of re-discovering it via a fresh crash every time.
                if let Some(info) = slot.current_model.clone() {
                    let gpu_layers = slot.gpu_layers;
                    let mut profile = load_gpu_profile(app, &info.id).await;
                    profile.last_safe_gpu_layers = Some(gpu_layers);
                    save_gpu_profile(app, &info, profile).await;
                }
                let _ = app.emit(
                    "model-ready",
                    json!({
                        "slot":     slot.index,
                        "model_id": model_id,
                        "port":     slot.port,
                        "cpu_mode": slot.cpu_mode,
                        "fallback_note": slot.fallback_note,
                    }),
                );
            }
        }
    }
}

fn should_retry_cpu_fallback(slot: &Slot, code: Option<i32>, detail: &str) -> bool {
    if slot.gpu_ladder.is_empty()
        || slot.load_attempt >= MAX_MODEL_LOAD_ATTEMPTS
        || !slot.inference_mode.allows_cpu_fallback()
    {
        return false;
    }

    is_gpu_crash_with_context(code, detail)
}

/// True for a confirmed GPU-crash code (`is_gpu_memory_crash`) on its own,
/// or for any other Windows `STATUS_*` code in the broader
/// `0xC0000000..=0xC0000FFF` range *only if* corroborated by GPU-specific
/// log text. That broad range covers hundreds of unrelated exceptions
/// (divide-by-zero, illegal instruction, etc.) — previously any code in it
/// was treated as a GPU crash unconditionally, which could misclassify an
/// unrelated crash as a GPU issue.
fn is_gpu_crash_with_context(code: Option<i32>, detail: &str) -> bool {
    if code.is_some_and(is_gpu_memory_crash) {
        return true;
    }
    let in_broad_status_range = code.is_some_and(|c| (0xC0000000..=0xC0000FFF).contains(&(c as u32)));
    in_broad_status_range && is_gpu_memory_log(detail)
}

fn is_gpu_memory_log(detail: &str) -> bool {
    let d = detail.to_lowercase();
    d.contains("status_access_violation")
        || d.contains("status_stack_buffer_overrun")
        || d.contains("erroroutofdevicememory")
        || d.contains("unable to allocate vulkan")
        || d.contains("vk::device::allocatememory")
}

fn classify_model_load_error(code: Option<i32>, detail: &str, status_text: String) -> String {
    if is_model_file_error(detail) {
        return format!("Model file error: {}", detail.trim());
    }

    match code {
        Some(c) if is_gpu_memory_crash(c) => {
            if detail.is_empty() {
                format!("GPU memory fault ({c:#010X})")
            } else {
                format!("GPU memory fault ({c:#010X}): {detail}")
            }
        }
        Some(c) => {
            if detail.is_empty() {
                format!("process exited with {c:#010X} ({status_text})")
            } else {
                format!("process exited with {c:#010X} ({status_text}): {detail}")
            }
        }
        None => {
            if detail.is_empty() {
                format!("process exited with {status_text}")
            } else {
                format!("process exited with {status_text}: {detail}")
            }
        }
    }
}

fn is_model_file_error(detail: &str) -> bool {
    let d = detail.to_lowercase();
    d.contains("failed to load model")
        || d.contains("no such file")
        || d.contains("cannot find the file")
        || d.contains("invalid gguf")
        || d.contains("corrupt")
}

/// Windows NTSTATUS codes confirmed, via real crash reproduction on this
/// app's GPU backend, to indicate a GPU driver/Vulkan fault:
/// `STATUS_ACCESS_VIOLATION` and `STATUS_STACK_BUFFER_OVERRUN`. Trusted on
/// their own, without needing log-text corroboration — see
/// `is_gpu_crash_with_context` for the broader, corroboration-gated check
/// used by the actual retry decision.
fn is_gpu_memory_crash(code: i32) -> bool {
    matches!(code as u32, 0xC0000005 | 0xC0000409)
}

// ── Queue drain ───────────────────────────────────────────────────────────────

// Fairness counter: alternates between "workspace" and "assistant" sources.
static FAIRNESS_TOGGLE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

async fn drain_queue(
    queue: &mut VecDeque<InferRequest>,
    slots: &mut Vec<Slot>,
    cmd_tx: &mpsc::UnboundedSender<Cmd>,
    client: &Client,
    app: &AppHandle,
) {
    while !queue.is_empty() {
        // Check if both sources are present — if so, apply round-robin
        let has_workspace = queue.iter().any(|r| r.source == "workspace");
        let has_assistant = queue.iter().any(|r| r.source == "assistant");

        let chosen_idx = if has_workspace && has_assistant {
            let toggle = FAIRNESS_TOGGLE.fetch_xor(1, Ordering::Relaxed);
            let prefer = if toggle == 0 {
                "workspace"
            } else {
                "assistant"
            };
            queue
                .iter()
                .position(|r| r.source == prefer)
                .or_else(|| Some(0))
        } else {
            Some(0)
        };

        if let Some(qi) = chosen_idx {
            let req = queue.remove(qi).unwrap();
            let mid = req.model_id.as_deref();
            if let Some(slot_idx) = best_ready_slot(slots, mid) {
                dispatch(slot_idx, req, slots, cmd_tx, client, app);
            } else {
                // No slot available — put back at front and stop draining
                queue.push_front(req);
                break;
            }
        } else {
            break;
        }
    }
}

fn dispatch(
    idx: usize,
    req: InferRequest,
    slots: &mut Vec<Slot>,
    cmd_tx: &mpsc::UnboundedSender<Cmd>,
    client: &Client,
    app: &AppHandle,
) {
    let slot = &mut slots[idx];
    let model_id = slot.state.model_id().unwrap_or("").to_string();
    slot.state = SlotState::Busy { model_id };
    slot.last_used = Instant::now();
    slot.total_requests += 1;

    let url = slot.base_url.clone();
    let client2 = client.clone();
    let app2 = app.clone();
    let notify = cmd_tx.clone();

    tauri::async_runtime::spawn(async move {
        let result = infer(
            req.messages,
            req.max_tokens,
            req.overrides,
            &url,
            &client2,
            req.stream_tx,
            req.cancel_flag,
            &app2,
        )
        .await;
        let _ = req.resp_tx.send(result);
        let _ = notify.send(Cmd::SlotFreed(idx));
    });
}

// ── Inference HTTP call ───────────────────────────────────────────────────────

async fn infer(
    messages: Vec<serde_json::Value>,
    max_tokens: u32,
    overrides: Option<InferenceOverrides>,
    base_url: &str,
    client: &Client,
    stream_tx: Option<mpsc::UnboundedSender<String>>,
    cancel_flag: Option<Arc<AtomicBool>>,
    app: &AppHandle,
) -> Result<(String, InferStats), String> {
    let ov = overrides.unwrap_or_default();
    let temperature = ov.temperature.unwrap_or(0.7);

    let mut body = json!({
        "model":          "local",
        "messages":       messages,
        "stream":         true,
        "temperature":    temperature,
        "max_tokens":     max_tokens,
        "stream_options": { "include_usage": true },
    });

    // Apply optional sampling overrides.
    let obj = body.as_object_mut().unwrap();
    if let Some(v) = ov.top_p {
        obj.insert("top_p".into(), json!(v));
    }
    if let Some(v) = ov.top_k {
        obj.insert("top_k".into(), json!(v));
    }
    if let Some(v) = ov.min_p {
        obj.insert("min_p".into(), json!(v));
    }
    if let Some(v) = ov.repeat_penalty {
        obj.insert("repeat_penalty".into(), json!(v));
    }
    if let Some(v) = ov.presence_penalty {
        obj.insert("presence_penalty".into(), json!(v));
    }
    if let Some(v) = ov.frequency_penalty {
        obj.insert("frequency_penalty".into(), json!(v));
    }
    if !ov.stop_sequences.is_empty() {
        obj.insert("stop".into(), json!(ov.stop_sequences));
    }

    let resp = client
        .post(format!("{}/v1/chat/completions", base_url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("llama-server request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp
            .text()
            .await
            .unwrap_or_else(|_| String::from("<no response body>"));
        let detail = body_text.chars().take(320).collect::<String>();
        return Err(format!("llama-server returned HTTP {status}: {detail}"));
    }

    let mut full = String::new();
    let mut fallback_tokens = 0u32;
    let mut prompt_tokens = 0u32;
    let mut completion_tokens = 0u32;
    let mut got_usage = false;
    let start = Instant::now();
    let mut first_token_at: Option<Duration> = None;
    let mut last_speed = Instant::now();
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        if cancel_flag
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            return Err("Generation cancelled by user".to_string());
        }

        let chunk = chunk.map_err(|e| e.to_string())?;
        for line in String::from_utf8_lossy(&chunk).lines() {
            if cancel_flag
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::Relaxed))
            {
                return Err("Generation cancelled by user".to_string());
            }

            let Some(data) = line.strip_prefix("data: ") else {
                continue;
            };
            if data.trim() == "[DONE]" {
                break;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                // Capture usage stats from the final streaming chunk
                if let Some(usage) = v.get("usage").filter(|u| !u.is_null()) {
                    prompt_tokens = usage["prompt_tokens"].as_u64().unwrap_or(0) as u32;
                    completion_tokens = usage["completion_tokens"].as_u64().unwrap_or(0) as u32;
                    got_usage = true;
                }
                if let Some(c) = v["choices"][0]["delta"]["content"].as_str() {
                    if first_token_at.is_none() {
                        first_token_at = Some(start.elapsed());
                    }
                    full.push_str(c);
                    fallback_tokens += 1;
                    match &stream_tx {
                        Some(tx) => {
                            let _ = tx.send(c.to_string());
                        }
                        None => {
                            let _ = app.emit("token-stream", c);
                        }
                    }
                    if last_speed.elapsed() >= Duration::from_secs(2) {
                        let elapsed = start.elapsed().as_secs_f64().max(0.001);
                        let tps = (fallback_tokens as f64 / elapsed) as u32;
                        let _ = app.emit("token-speed", tps);
                        last_speed = Instant::now();
                    }
                }
            }
        }
    }

    if !got_usage {
        completion_tokens = fallback_tokens;
    }

    let total_ms = start.elapsed().as_millis() as u64;
    let tps = if total_ms > 0 {
        (completion_tokens as f64 / (total_ms as f64 / 1000.0)) as f32
    } else {
        0.0
    };

    let stats = InferStats {
        prompt_tokens,
        completion_tokens,
        tokens_per_second: tps,
        time_to_first_token_ms: first_token_at.map(|d| d.as_millis() as u64).unwrap_or(0),
        total_time_ms: total_ms,
    };

    Ok((full, stats))
}

// ── Status helpers ────────────────────────────────────────────────────────────

fn build_status(slots: &[Slot], queue: &VecDeque<InferRequest>) -> OrchestratorStatus {
    let mut sys = System::new();
    sys.refresh_memory();
    OrchestratorStatus {
        slots: slots
            .iter()
            .map(|s| SlotStatus {
                index: s.index,
                port: s.port,
                state: s.state.clone(),
                requests: s.total_requests,
                idle_secs: s.last_used.elapsed().as_secs(),
                load_elapsed_secs: s.load_started.map(|t| t.elapsed().as_secs()),
                fallback_note: s.fallback_note.clone(),
                cpu_mode: s.cpu_mode,
                inference_mode: s.inference_mode.clone(),
            })
            .collect(),
        queue_depth: queue.len(),
        total_ram_mb: sys.total_memory() / (1024 * 1024),
        free_ram_mb: sys.available_memory() / (1024 * 1024),
    }
}

fn is_false(v: &bool) -> bool {
    !*v
}

fn emit_status(slots: &[Slot], queue: &VecDeque<InferRequest>, app: &AppHandle) {
    let _ = app.emit("orchestrator-status", build_status(slots, queue));
}

// ── System helpers ────────────────────────────────────────────────────────────

fn decide_slot_count() -> usize {
    let mut sys = System::new();
    sys.refresh_memory();
    let ram_gb = sys.total_memory() / (1024 * 1024 * 1024);
    // 2 slots if ≥ 16 GB RAM, else 1
    if ram_gb >= 16 {
        2
    } else {
        1
    }
}

fn thread_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(8)
}

/// Returns true if a discrete GPU (NVIDIA / AMD / Intel Arc) is present.
/// Used to decide whether to pass `--n-gpu-layers -1` to llama-server.
pub(crate) fn has_discrete_gpu() -> bool {
    #[cfg(target_os = "windows")]
    {
        let looks_discrete = |s: &str| {
            let lower = s.to_lowercase();
            lower.contains("nvidia")
                || lower.contains("radeon")
                || lower.contains("amd")
                || lower.contains("intel arc")
                || lower.contains("intel xe")
        };

        if let Ok(out) = {
            let mut c = std::process::Command::new("wmic");
            c.args(["path", "win32_VideoController", "get", "name"]);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                c.creation_flags(0x0800_0000);
            }
            c.output()
        } {
            let s = String::from_utf8_lossy(&out.stdout);
            if looks_discrete(&s) {
                return true;
            }
        }

        // WMIC can be unavailable/deprecated on some Windows installs.
        // Use a PowerShell CIM fallback so GPU-first remains reliable.
        if let Ok(out) = {
            let mut c = std::process::Command::new("powershell");
            c.args([
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name",
            ]);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                c.creation_flags(0x0800_0000);
            }
            c.output()
        } {
            let s = String::from_utf8_lossy(&out.stdout);
            if looks_discrete(&s) {
                return true;
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(out) = std::process::Command::new("lspci").output() {
            let s = String::from_utf8_lossy(&out.stdout).to_lowercase();
            return s.contains("nvidia") || s.contains("amd") || s.contains("radeon");
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn spawn_ok_server() -> (u16, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let port = listener.local_addr().expect("local addr").port();

        let handle = tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                let _ = socket
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK",
                    )
                    .await;
                let _ = socket.shutdown().await;
            }
        });

        (port, handle)
    }

    #[tokio::test]
    async fn active_slot_url_resolves_from_loading_health_probe() {
        let (port, handle) = spawn_ok_server().await;
        let status = OrchestratorStatus {
            slots: vec![SlotStatus {
                index: 0,
                port,
                state: SlotState::Loading {
                    model_id: "Bonsai-1.7B".to_string(),
                    load_pct: Some(100),
                },
                requests: 0,
                idle_secs: 0,
                load_elapsed_secs: Some(1),
                fallback_note: None,
                cpu_mode: false,
                inference_mode: InferenceMode::default(),
            }],
            queue_depth: 0,
            total_ram_mb: 0,
            free_ram_mb: 0,
        };

        let client = Client::new();
        let resolved = resolve_active_slot_url(&status, &client).await;
        assert_eq!(resolved, Some(format!("http://127.0.0.1:{port}")));

        handle.abort();
    }

    #[test]
    fn gpu_crash_triggers_cpu_fallback() {
        assert!(is_gpu_memory_crash(0xC0000005u32 as i32));
        assert!(is_gpu_memory_crash(0xC0000409u32 as i32));
        assert!(!is_gpu_memory_crash(0));
        assert!(!is_gpu_memory_crash(1));
    }

    #[test]
    fn slot_lifecycle_transitions_are_valid() {
        let mut slot = Slot::new(0);
        assert!(matches!(slot.state, SlotState::Empty));

        slot.state = SlotState::Loading {
            model_id: "test-model".to_string(),
            load_pct: Some(25),
        };
        assert!(slot.state.is_loading());

        slot.state = SlotState::Ready {
            model_id: "test-model".to_string(),
        };
        assert!(slot.state.is_ready());

        slot.state = SlotState::Busy {
            model_id: "test-model".to_string(),
        };
        assert!(matches!(slot.state, SlotState::Busy { .. }));

        slot.kill();
        assert!(slot.state.is_empty());
    }

    #[test]
    fn cpu_fallback_retry_policy_respects_mode_and_attempts() {
        let mut slot = Slot::new(0);
        slot.cpu_mode = false;
        slot.load_attempt = 1;
        slot.inference_mode = InferenceMode::Auto;
        // A non-empty ladder means there's a fallback rung left to try.
        slot.gpu_ladder = vec![PlacementPlan::FullGpu(0)];
        assert!(should_retry_cpu_fallback(
            &slot,
            Some(0xC0000409u32 as i32),
            "STATUS_STACK_BUFFER_OVERRUN"
        ));

        slot.inference_mode = InferenceMode::GpuOnly;
        assert!(!should_retry_cpu_fallback(
            &slot,
            Some(0xC0000409u32 as i32),
            "STATUS_STACK_BUFFER_OVERRUN"
        ));

        slot.inference_mode = InferenceMode::Auto;
        slot.load_attempt = MAX_MODEL_LOAD_ATTEMPTS;
        assert!(!should_retry_cpu_fallback(
            &slot,
            Some(0xC0000409u32 as i32),
            "STATUS_STACK_BUFFER_OVERRUN"
        ));

        // Empty ladder (no fallback rung left) — must not retry even with
        // attempts remaining.
        slot.load_attempt = 1;
        slot.gpu_ladder = vec![];
        assert!(!should_retry_cpu_fallback(
            &slot,
            Some(0xC0000409u32 as i32),
            "STATUS_STACK_BUFFER_OVERRUN"
        ));
    }

    #[test]
    fn is_gpu_crash_with_context_requires_corroboration_outside_confirmed_codes() {
        // Confirmed codes are trusted on their own.
        assert!(is_gpu_crash_with_context(Some(0xC0000409u32 as i32), ""));
        assert!(is_gpu_crash_with_context(Some(0xC0000005u32 as i32), ""));

        // Codes elsewhere in the broad STATUS_* range (divide-by-zero,
        // illegal instruction) must NOT be misclassified as GPU crashes
        // without log corroboration.
        assert!(!is_gpu_crash_with_context(Some(0xC0000094u32 as i32), ""));
        assert!(!is_gpu_crash_with_context(Some(0xC000001Du32 as i32), ""));

        // But corroborated by GPU-specific log text, they count.
        assert!(is_gpu_crash_with_context(
            Some(0xC0000094u32 as i32),
            "ErrorOutOfDeviceMemory: vk::Device::allocateMemory failed"
        ));
    }

    #[test]
    fn build_gpu_ladder_steps_down_before_giving_up() {
        let ladder = build_gpu_ladder(PlacementPlan::FullGpu(20), &InferenceMode::Auto);
        assert_eq!(
            ladder,
            vec![
                PlacementPlan::FullGpu(20),
                PlacementPlan::FullGpu(10),
                PlacementPlan::FullGpu(0),
            ]
        );
    }

    #[test]
    fn build_gpu_ladder_respects_gpu_only_no_fallback() {
        let ladder = build_gpu_ladder(PlacementPlan::FullGpu(20), &InferenceMode::GpuOnly);
        assert_eq!(ladder, vec![PlacementPlan::FullGpu(20)]);
    }

    #[test]
    fn build_gpu_ladder_preserves_tensor_override_then_falls_back() {
        let plan = PlacementPlan::TensorOverride {
            ot_rules: vec!["blk\\.\\d+\\.ffn_gate$=CPU".to_string()],
            gpu_layers: 24,
        };
        let ladder = build_gpu_ladder(plan.clone(), &InferenceMode::Auto);
        assert_eq!(ladder[0], plan);
        assert_eq!(ladder[1], PlacementPlan::FullGpu(12));
        assert_eq!(ladder[2], PlacementPlan::FullGpu(0));
    }

    #[test]
    fn placement_plan_cli_resolves_expected_args() {
        assert_eq!(placement_plan_cli(&PlacementPlan::FullGpu(10)), (10, vec![], None));
        assert_eq!(placement_plan_cli(&PlacementPlan::CpuOnly), (0, vec![], None));
        assert_eq!(
            placement_plan_cli(&PlacementPlan::CpuMoe {
                n_cpu_moe: 4,
                gpu_layers: 20
            }),
            (20, vec![], Some(4))
        );
    }

    #[test]
    fn apply_mode_override_lets_explicit_choice_win() {
        assert_eq!(
            apply_mode_override(PlacementPlan::FullGpu(20), &InferenceMode::CpuOnly),
            PlacementPlan::FullGpu(0)
        );
        assert_eq!(
            apply_mode_override(PlacementPlan::CpuOnly, &InferenceMode::GpuOnly),
            PlacementPlan::FullGpu(1)
        );
        assert_eq!(
            apply_mode_override(PlacementPlan::FullGpu(20), &InferenceMode::Hybrid { gpu_layers: 7 }),
            PlacementPlan::FullGpu(7)
        );
    }

    #[test]
    fn empty_or_evict_prefers_empty_then_lru_ready() {
        let mut slots = vec![Slot::new(0), Slot::new(1), Slot::new(2)];

        slots[0].state = SlotState::Ready {
            model_id: "a".to_string(),
        };
        slots[1].state = SlotState::Busy {
            model_id: "b".to_string(),
        };
        slots[2].state = SlotState::Empty;
        assert_eq!(empty_or_evict(&mut slots), 2);

        slots[2].state = SlotState::Ready {
            model_id: "c".to_string(),
        };
        slots[0].last_used = Instant::now() - Duration::from_secs(120);
        slots[2].last_used = Instant::now() - Duration::from_secs(30);
        assert_eq!(empty_or_evict(&mut slots), 0);
    }

    #[test]
    fn slot_new_uses_unique_ports_for_small_batch() {
        let mut seen = HashSet::new();
        for i in 0..8 {
            let s = Slot::new(i);
            assert!(seen.insert(s.port), "duplicate port allocated: {}", s.port);
            assert!((30_000..50_000).contains(&s.port));
        }
    }
}
