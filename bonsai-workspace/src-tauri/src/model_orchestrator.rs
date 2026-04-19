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

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::StreamExt;
use rand::Rng;
use reqwest::Client;
use serde::Serialize;
use serde_json::json;
use sysinfo::System;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::bootstrap;
use crate::model_registry::{ModelInfo, ModelRegistry};

const MODEL_LOAD_POLL_INTERVAL_MS: u64 = 500;
#[cfg(target_os = "android")]
const MODEL_LOAD_TIMEOUT_SECS: u64 = 420;
#[cfg(not(target_os = "android"))]
const MODEL_LOAD_TIMEOUT_SECS: u64 = 240;
const MODEL_LOAD_MAX_POLLS: u64 = (MODEL_LOAD_TIMEOUT_SECS * 1000) / MODEL_LOAD_POLL_INTERVAL_MS;

// ── Slot state ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SlotState {
    Empty,
    Loading { model_id: String },
    Ready   { model_id: String },
    Busy    { model_id: String },
    Crashed { model_id: String, error: String },
}

impl SlotState {
    pub fn model_id(&self) -> Option<&str> {
        match self {
            Self::Loading { model_id }
            | Self::Ready { model_id }
            | Self::Busy  { model_id }
            | Self::Crashed { model_id, .. } => Some(model_id),
            Self::Empty => None,
        }
    }
    pub fn is_ready(&self)   -> bool { matches!(self, Self::Ready { .. }) }
    pub fn is_empty(&self)   -> bool { matches!(self, Self::Empty) }
    pub fn is_loading(&self) -> bool { matches!(self, Self::Loading { .. }) }
}

// ── Slot ──────────────────────────────────────────────────────────────────────

struct Slot {
    index:          usize,
    port:           u16,
    base_url:       String,
    state:          SlotState,
    process:        Option<std::process::Child>,
    last_used:      Instant,
    total_requests: u64,
}

impl Slot {
    fn new(index: usize) -> Self {
        let port = rand::thread_rng().gen_range(30_000u16..50_000u16);
        Self {
            index,
            port,
            base_url: format!("http://127.0.0.1:{}", port),
            state: SlotState::Empty,
            process: None,
            last_used: Instant::now(),
            total_requests: 0,
        }
    }

    fn kill(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
        }
        self.state = SlotState::Empty;
    }
}

impl Drop for Slot {
    fn drop(&mut self) { self.kill(); }
}

// ── Public status types ───────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct SlotStatus {
    pub index:        usize,
    pub port:         u16,
    pub state:        SlotState,
    pub requests:     u64,
    pub idle_secs:    u64,
}

#[derive(Serialize, Clone)]
pub struct OrchestratorStatus {
    pub slots:        Vec<SlotStatus>,
    pub queue_depth:  usize,
    pub total_ram_mb: u64,
    pub free_ram_mb:  u64,
}

// ── Token stats ───────────────────────────────────────────────────────────────

#[derive(Serialize, Clone, Default, Debug)]
pub struct InferStats {
    pub prompt_tokens:          u32,
    pub completion_tokens:      u32,
    pub tokens_per_second:      f32,
    pub time_to_first_token_ms: u64,
    pub total_time_ms:          u64,
}

// ── Infer request ─────────────────────────────────────────────────────────────

pub struct InferRequest {
    /// Which model to use; None = any ready slot.
    pub model_id:   Option<String>,
    /// Full OpenAI-format message history (system + user + assistant turns).
    pub messages:   Vec<serde_json::Value>,
    pub max_tokens: u32,
    /// If Some, tokens are streamed here instead of via the app event bus.
    pub stream_tx:  Option<mpsc::UnboundedSender<String>>,
    /// Optional cancellation flag set by the UI to stop active generation.
    pub cancel_flag: Option<Arc<AtomicBool>>,
    pub resp_tx:    oneshot::Sender<Result<(String, InferStats), String>>,
    /// Request source tag for fairness scheduling ("workspace" | "assistant").
    pub source: &'static str,
}

// ── Internal command ──────────────────────────────────────────────────────────

enum Cmd {
    Infer(InferRequest),
    Load { model_id: String, resp_tx: oneshot::Sender<Result<(), String>> },
    Unload(usize),
    Status { resp_tx: oneshot::Sender<OrchestratorStatus> },
    RefreshRegistry,
    SlotFreed(usize),
}

// ── Public handle ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ModelOrchestrator {
    cmd_tx:   mpsc::UnboundedSender<Cmd>,
    registry: Arc<Mutex<ModelRegistry>>,
}

impl ModelOrchestrator {
    pub fn new(app: AppHandle) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Cmd>();
        let models_dir = bootstrap::models_dir(&app);
        let registry   = Arc::new(Mutex::new(ModelRegistry::scan(&models_dir)));

        let reg2    = registry.clone();
        let cmd_tx2 = cmd_tx.clone();
        tauri::async_runtime::spawn(async move {
            event_loop(cmd_rx, cmd_tx2, reg2, app).await;
        });

        Self { cmd_tx, registry }
    }

    /// Submit a streaming inference request.
    pub fn infer(&self, req: InferRequest) -> Result<(), String> {
        self.cmd_tx.send(Cmd::Infer(req)).map_err(|_| "orchestrator offline".into())
    }

    /// Load a model by ID (non-blocking; use the returned receiver to await readiness).
    pub fn load(&self, model_id: String) -> oneshot::Receiver<Result<(), String>> {
        let (tx, rx) = oneshot::channel();
        let _ = self.cmd_tx.send(Cmd::Load { model_id, resp_tx: tx });
        rx
    }

    pub fn unload(&self, slot: usize) {
        let _ = self.cmd_tx.send(Cmd::Unload(slot));
    }

    pub fn refresh_registry(&self) {
        let _ = self.cmd_tx.send(Cmd::RefreshRegistry);
    }

    pub async fn status(&self) -> OrchestratorStatus {
        let (tx, rx) = oneshot::channel();
        let _ = self.cmd_tx.send(Cmd::Status { resp_tx: tx });
        rx.await.unwrap_or_else(|_| OrchestratorStatus {
            slots: vec![], queue_depth: 0, total_ram_mb: 0, free_ram_mb: 0,
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
        status.slots.iter()
            .find(|s| s.state.is_ready())
            .map(|s| format!("http://127.0.0.1:{}", s.port))
    }
}

// ── Event loop ────────────────────────────────────────────────────────────────

async fn event_loop(
    mut rx:     mpsc::UnboundedReceiver<Cmd>,
    cmd_tx:     mpsc::UnboundedSender<Cmd>,
    registry:   Arc<Mutex<ModelRegistry>>,
    app:        AppHandle,
) {
    let n_slots = decide_slot_count();
    let mut slots: Vec<Slot> = (0..n_slots).map(Slot::new).collect();
    let mut queue: VecDeque<InferRequest> = VecDeque::new();

    let client = Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .unwrap_or_default();

    // Auto-load the first model at startup
    {
        let reg = registry.lock().await;
        if let Some(info) = reg.models.first().cloned() {
            drop(reg);
            spawn_model(&mut slots[0], &info, &app);
        }
    }

    loop {
        tokio::select! {
            cmd = rx.recv() => match cmd {
                None => break,
                Some(c) => handle_cmd(c, &mut slots, &mut queue, &cmd_tx, &registry, &client, &app).await,
            },
            // Periodic: advance queue + poll Loading slots for readiness
            _ = tokio::time::sleep(Duration::from_millis(250)) => {
                poll_loading_slots(&mut slots, &client, &app).await;
                drain_queue(&mut queue, &mut slots, &cmd_tx, &client, &app).await;
                emit_status(&slots, &queue, &app);
            }
        }
    }
}

async fn handle_cmd(
    cmd:      Cmd,
    slots:    &mut Vec<Slot>,
    queue:    &mut VecDeque<InferRequest>,
    cmd_tx:   &mpsc::UnboundedSender<Cmd>,
    registry: &Arc<Mutex<ModelRegistry>>,
    client:   &Client,
    app:      &AppHandle,
) {
    match cmd {
        Cmd::Infer(req) => {
            let mid = req.model_id.as_deref();
            if let Some(idx) = best_ready_slot(slots, mid) {
                dispatch(idx, req, slots, cmd_tx, client, app);
            } else {
                // No ready slot — if a suitable model isn't loading, start it
                if let Some(mid_owned) = req.model_id.clone() {
                    maybe_start_load(mid_owned, slots, registry, app).await;
                }
                queue.push_back(req);
            }
        }

        Cmd::Load { model_id, resp_tx } => {
            // Already ready?
            if slots.iter().any(|s| s.state.model_id() == Some(&model_id) && s.state.is_ready()) {
                let _ = resp_tx.send(Ok(()));
                return;
            }
            // Already loading?
            if slots.iter().any(|s| s.state.model_id() == Some(&model_id) && s.state.is_loading()) {
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
                None => { let _ = resp_tx.send(Err(format!("model {model_id} not in registry"))); }
                Some(info) => {
                    let idx = empty_or_evict(slots);
                    spawn_model(&mut slots[idx], &info, app);
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

        Cmd::RefreshRegistry => {
            // Re-scan the models directory
            let models_dir = bootstrap::models_dir(app);
            {
                let mut reg = registry.lock().await;
                reg.refresh(&models_dir);
            }
            let _ = app.emit("registry-updated", ());
            // If all slots are empty/crashed, try to auto-load the first model
            let all_idle = slots.iter().all(|s| {
                matches!(s.state, SlotState::Empty | SlotState::Crashed { .. })
            });
            if all_idle {
                let reg = registry.lock().await;
                if let Some(info) = reg.models.first().cloned() {
                    drop(reg);
                    spawn_model(&mut slots[0], &info, app);
                }
            }
        }

        Cmd::SlotFreed(idx) => {
            if let Some(slot) = slots.get_mut(idx) {
                if let SlotState::Busy { model_id } = &slot.state.clone() {
                    slot.state = SlotState::Ready { model_id: model_id.clone() };
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
    Err(format!("model load timeout after {}s", MODEL_LOAD_TIMEOUT_SECS))
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

fn spawn_model(slot: &mut Slot, info: &ModelInfo, app: &AppHandle) {
    slot.kill();
    slot.state = SlotState::Loading { model_id: info.id.clone() };

    let exe = bootstrap::llama_exe(app);
    if !exe.exists() {
        slot.state = SlotState::Crashed {
            model_id: info.id.clone(),
            error: "llama-server binary not found — bootstrap required".into(),
        };
        return;
    }

    let dir      = exe.parent().unwrap_or(&exe).to_path_buf();
    let port_str = slot.port.to_string();
    let ctx      = info.context_length.clamp(512, 8192).to_string();
    let threads  = thread_count().to_string();

    // If the llama-server binary name contains "vulkan" it was compiled with Vulkan
    // support; offload all layers to GPU. Otherwise fall back to CPU-only.
    let exe_name = exe.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();
    let n_gpu_layers = if exe_name.contains("vulkan") || has_discrete_gpu() { "-1" } else { "0" };

    let mut cmd = std::process::Command::new(&exe);
    cmd.args([
        "--port",    &port_str,
        "--host",    "127.0.0.1",
        "--model",   &info.path.to_string_lossy(),
        "--ctx-size", &ctx,
        "--threads", &threads,
        "--n-gpu-layers", n_gpu_layers,
        "--log-disable",
    ])
    .current_dir(&dir)
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null());

    #[cfg(windows)] {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }

    match cmd.spawn() {
        Ok(child) => { slot.process = Some(child); }
        Err(e)    => {
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
        if let Some(i) = slots.iter().position(|s| {
            s.state.is_ready() && s.state.model_id() == Some(mid)
        }) { return Some(i); }
    }
    // Any ready slot
    slots.iter().position(|s| s.state.is_ready())
}

fn empty_or_evict(slots: &mut Vec<Slot>) -> usize {
    if let Some(i) = slots.iter().position(|s| s.state.is_empty()) { return i; }
    // LRU eviction among Ready/Crashed slots (not Busy or Loading)
    slots.iter()
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
    app: &AppHandle,
) {
    // Don't load if already loading/ready
    if slots.iter().any(|s| s.state.model_id() == Some(&model_id)) { return; }
    let reg = registry.lock().await;
    if let Some(info) = reg.models.iter().find(|m| m.id == model_id).cloned() {
        drop(reg);
        let idx = empty_or_evict(slots);
        spawn_model(&mut slots[idx], &info, app);
    }
}

// ── Health polling ────────────────────────────────────────────────────────────

async fn poll_loading_slots(slots: &mut Vec<Slot>, client: &Client, app: &AppHandle) {
    for slot in slots.iter_mut() {
        if let SlotState::Loading { model_id } = &slot.state.clone() {
            // Check if the process has exited unexpectedly
            if let Some(ref mut child) = slot.process {
                if let Ok(Some(status)) = child.try_wait() {
                    slot.state = SlotState::Crashed {
                        model_id: model_id.clone(),
                        error: format!("process exited with {status}"),
                    };
                    continue;
                }
            }
            // Probe readiness endpoints (/health and /v1/models fallback).
            let ok = probe_model_ready(client, &slot.base_url).await;
            if ok {
                slot.state = SlotState::Ready { model_id: model_id.clone() };
                let _ = app.emit("model-ready", json!({
                    "slot":     slot.index,
                    "model_id": model_id,
                    "port":     slot.port,
                }));
            }
        }
    }
}

// ── Queue drain ───────────────────────────────────────────────────────────────

// Fairness counter: alternates between "workspace" and "assistant" sources.
static FAIRNESS_TOGGLE: std::sync::atomic::AtomicU8 =
    std::sync::atomic::AtomicU8::new(0);

async fn drain_queue(
    queue:  &mut VecDeque<InferRequest>,
    slots:  &mut Vec<Slot>,
    cmd_tx: &mpsc::UnboundedSender<Cmd>,
    client: &Client,
    app:    &AppHandle,
) {
    while !queue.is_empty() {
        // Check if both sources are present — if so, apply round-robin
        let has_workspace  = queue.iter().any(|r| r.source == "workspace");
        let has_assistant  = queue.iter().any(|r| r.source == "assistant");

        let chosen_idx = if has_workspace && has_assistant {
            let toggle = FAIRNESS_TOGGLE.fetch_xor(1, Ordering::Relaxed);
            let prefer = if toggle == 0 { "workspace" } else { "assistant" };
            queue.iter().position(|r| r.source == prefer)
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
    idx:    usize,
    req:    InferRequest,
    slots:  &mut Vec<Slot>,
    cmd_tx: &mpsc::UnboundedSender<Cmd>,
    client: &Client,
    app:    &AppHandle,
) {
    let slot = &mut slots[idx];
    let model_id = slot.state.model_id().unwrap_or("").to_string();
    slot.state = SlotState::Busy { model_id };
    slot.last_used = Instant::now();
    slot.total_requests += 1;

    let url      = slot.base_url.clone();
    let client2  = client.clone();
    let app2     = app.clone();
    let notify   = cmd_tx.clone();

    tauri::async_runtime::spawn(async move {
        let result = infer(
            req.messages,
            req.max_tokens,
            &url,
            &client2,
            req.stream_tx,
            req.cancel_flag,
            &app2,
        ).await;
        let _ = req.resp_tx.send(result);
        let _ = notify.send(Cmd::SlotFreed(idx));
    });
}

// ── Inference HTTP call ───────────────────────────────────────────────────────

async fn infer(
    messages:   Vec<serde_json::Value>,
    max_tokens: u32,
    base_url:   &str,
    client:     &Client,
    stream_tx:  Option<mpsc::UnboundedSender<String>>,
    cancel_flag: Option<Arc<AtomicBool>>,
    app:        &AppHandle,
) -> Result<(String, InferStats), String> {
    let body = json!({
        "model":          "local",
        "messages":       messages,
        "stream":         true,
        "temperature":    0.7,
        "max_tokens":     max_tokens,
        "stream_options": { "include_usage": true },
    });

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

    let mut full                 = String::new();
    let mut fallback_tokens      = 0u32;
    let mut prompt_tokens        = 0u32;
    let mut completion_tokens    = 0u32;
    let mut got_usage            = false;
    let     start                = Instant::now();
    let mut first_token_at: Option<Duration> = None;
    let mut last_speed           = Instant::now();
    let mut stream               = resp.bytes_stream();

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

            let Some(data) = line.strip_prefix("data: ") else { continue };
            if data.trim() == "[DONE]" { break; }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                // Capture usage stats from the final streaming chunk
                if let Some(usage) = v.get("usage").filter(|u| !u.is_null()) {
                    prompt_tokens     = usage["prompt_tokens"].as_u64().unwrap_or(0) as u32;
                    completion_tokens = usage["completion_tokens"].as_u64().unwrap_or(0) as u32;
                    got_usage         = true;
                }
                if let Some(c) = v["choices"][0]["delta"]["content"].as_str() {
                    if first_token_at.is_none() {
                        first_token_at = Some(start.elapsed());
                    }
                    full.push_str(c);
                    fallback_tokens += 1;
                    match &stream_tx {
                        Some(tx) => { let _ = tx.send(c.to_string()); }
                        None     => { let _ = app.emit("token-stream", c); }
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
        tokens_per_second:      tps,
        time_to_first_token_ms: first_token_at.map(|d| d.as_millis() as u64).unwrap_or(0),
        total_time_ms:          total_ms,
    };

    Ok((full, stats))
}

// ── Status helpers ────────────────────────────────────────────────────────────

fn build_status(slots: &[Slot], queue: &VecDeque<InferRequest>) -> OrchestratorStatus {
    let mut sys = System::new();
    sys.refresh_memory();
    OrchestratorStatus {
        slots: slots.iter().map(|s| SlotStatus {
            index:     s.index,
            port:      s.port,
            state:     s.state.clone(),
            requests:  s.total_requests,
            idle_secs: s.last_used.elapsed().as_secs(),
        }).collect(),
        queue_depth:  queue.len(),
        total_ram_mb: sys.total_memory() / (1024 * 1024),
        free_ram_mb:  sys.available_memory() / (1024 * 1024),
    }
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
    if ram_gb >= 16 { 2 } else { 1 }
}

fn thread_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(8)
}

/// Returns true if a discrete GPU (NVIDIA / AMD / Intel Arc) is present.
/// Used to decide whether to pass `--n-gpu-layers -1` to llama-server.
fn has_discrete_gpu() -> bool {
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

        if let Ok(out) = std::process::Command::new("wmic")
            .args(["path", "win32_VideoController", "get", "name"])
            .output()
        {
            let s = String::from_utf8_lossy(&out.stdout);
            if looks_discrete(&s) {
                return true;
            }
        }

        // WMIC can be unavailable/deprecated on some Windows installs.
        // Use a PowerShell CIM fallback so GPU-first remains reliable.
        if let Ok(out) = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name",
            ])
            .output()
        {
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
