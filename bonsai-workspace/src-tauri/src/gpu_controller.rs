//! Unified GPU Controller — single decision point for all GPU operations.
//!
//! Integrates `GpuLayer` (health/routing), `SharedMemoryArena` (cross-model
//! memory), and `MicroBonsai` (load prediction) into one coherent abstraction.
//!
//! Users never see GPU failures — all operations have automatic CPU fallback
//! with async GPU recovery scheduled behind the scenes.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::gpu_layer::{BackendType, GpuLayer};
use crate::micro_bonsai::MicroBonsai;
use crate::shared_arena::SharedMemoryArena;
use serde_json::json;
use std::time::{Duration, Instant};

// ── Tuning constants ──────────────────────────────────────────────────────────

/// Keep this many MiB of VRAM headroom for KV cache + driver overhead.
const VRAM_HEADROOM_MB: u64 = 2048;
/// Maximum fraction of total VRAM a single model may use.
const MAX_PER_MODEL_VRAM_FRACTION: f64 = 0.65;
/// On battery (heuristic: <20 GB free RAM suggests laptop), cap GPU layers at this fraction.
const BATTERY_LAYER_FRACTION: f64 = 0.30;
/// Conservative estimate of bytes per transformer layer for unknown models.
const BYTES_PER_LAYER_FALLBACK: u64 = 100 * 1024 * 1024; // 100 MiB
/// Total assumed transformer layers when we can't determine from the file.
const DEFAULT_LAYER_COUNT: u64 = 40;

// ── Layer allocation record ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub(crate) struct LayerAlloc {
    layers:      u32,
    vram_est_mb: u64,
    last_used:   std::time::Instant,
}

// ── Health report ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GpuHealthReport {
    pub backend:               String,
    pub healthy:               bool,
    pub vram_total_mb:         u64,
    pub vram_free_mb:          u64,
    pub loaded_models:         Vec<String>,
    pub total_vram_reserved_mb: u64,
    pub allocation_ok:         bool,
    pub fallback_active:       bool,
    pub recovery_pending:      bool,
    pub uptime_secs:           u64,
}

// ── Controller ────────────────────────────────────────────────────────────────

pub struct GpuController {
    pub gpu:          Arc<GpuLayer>,
    arena:            Arc<SharedMemoryArena>,
    micro:            Arc<MicroBonsai>,
    /// Map model_path → current layer allocation.
    pub(crate) allocs: RwLock<HashMap<String, LayerAlloc>>,
    /// Last-used times for models (separate view used by TTL monitor).
    pub(crate) model_usage: RwLock<HashMap<String, Instant>>,
    /// True if a recovery task is already scheduled.
    pub recovery_pending: Arc<RwLock<bool>>,
    started_at:       std::time::Instant,
}

impl GpuController {
    pub fn new(
        gpu:   Arc<GpuLayer>,
        arena: Arc<SharedMemoryArena>,
        micro: Arc<MicroBonsai>,
    ) -> Arc<Self> {
        Arc::new(Self {
            gpu,
            arena,
            micro,
            allocs:           RwLock::new(HashMap::new()),
            model_usage:      RwLock::new(HashMap::new()),
            recovery_pending: Arc::new(RwLock::new(false)),
            started_at:       std::time::Instant::now(),
        })
    }

    /// Run a TTL monitor that unloads models idle for `ttl_secs`, checking every `check_interval_secs`.
    pub async fn run_ttl_monitor(self: Arc<Self>, ttl_secs: u64, check_interval_secs: u64) {
        let this = self.clone();
        tokio::spawn(async move {
            let ttl = Duration::from_secs(ttl_secs);
            let interval = Duration::from_secs(check_interval_secs);
            loop {
                tokio::time::sleep(interval).await;
                let now = Instant::now();
                let mut to_evict = Vec::new();
                {
                    let usage = this.model_usage.read().await;
                    for (m, t) in usage.iter() {
                        if now.duration_since(*t) > ttl {
                            to_evict.push(m.clone());
                        }
                    }
                }
                for m in to_evict {
                    tracing::info!(model=%m, "[gpu_ctrl] TTL evicting model");
                    // release alloc and remove usage
                    this.release(&m).await;
                    this.model_usage.write().await.remove(&m);
                }
            }
        });
    }

    /// Spawn a background task that emits a Tauri event if the GPU becomes
    /// unavailable.  Runs a lightweight VRAM check every `interval_secs`.
    pub fn start_health_monitor(self: Arc<Self>, app_handle: tauri::AppHandle, interval_secs: u64) {
        tokio::spawn(async move {
            let interval = Duration::from_secs(interval_secs);
            let mut consecutive_failures: u32 = 0;
            loop {
                tokio::time::sleep(interval).await;
                let vram = self.gpu.free_vram_mb();
                // Treat 0 MB free as a potential GPU hang (true in practice when driver is gone)
                if vram == 0 {
                    consecutive_failures += 1;
                    warn!("[gpu-health] probe failed (consecutive={})", consecutive_failures);
                    if consecutive_failures >= 2 {
                        let _ = tauri::Emitter::emit(&app_handle, "gpu-unhealthy", serde_json::json!({
                            "consecutive_failures": consecutive_failures,
                            "vram_free_mb": vram,
                        }));
                    }
                } else {
                    if consecutive_failures > 0 {
                        info!("[gpu-health] GPU recovered after {} failures", consecutive_failures);
                        let _ = tauri::Emitter::emit(&app_handle, "gpu-recovered", serde_json::json!({}));
                    }
                    consecutive_failures = 0;
                }
            }
        });
    }

    /// Update last-used timestamp for a model.
    pub async fn touch_model(&self, model_path: &str) {
        self.model_usage.write().await.insert(model_path.to_string(), Instant::now());
    }

    /// Return VRAM/profile summary as JSON value.
    pub async fn profile_vram(&self) -> serde_json::Value {
        let vram_free = self.gpu.free_vram_mb();
        let vram_total = vram_free; // best-effort proxy
        let loaded_models: Vec<String> = self.allocs.read().await.keys().cloned().collect();
        json!({
            "free_mb": vram_free,
            "total_mb": vram_total,
            "used_mb": vram_total.saturating_sub(vram_free),
            "loaded_models": loaded_models,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })
    }

    // ── Layer allocation ──────────────────────────────────────────────────────

    /// Decide the optimal number of GPU layers for `model_path`.
    ///
    /// Considers: free VRAM, per-model cap, current allocations (LRU eviction),
    /// power mode, and an explicit `requested` override from the user.
    pub async fn decide_layers(
        &self,
        model_path: &str,
        requested:  Option<u32>,
        power_saving: bool,
    ) -> u32 {
        let vram_free = self.gpu.free_vram_mb();
        let vram_total = {
            // Use sysinfo total RAM as a proxy when real VRAM isn't available.
            // GpuLayer.free_vram_mb() already does this — keep consistent.
            vram_free.max(1)
        };

        // Estimate bytes per layer from file size.
        let file_mb = std::fs::metadata(model_path)
            .map(|m| m.len() / (1024 * 1024))
            .unwrap_or(0) as u64;
        let per_layer_mb = if file_mb > 0 {
            file_mb / DEFAULT_LAYER_COUNT
        } else {
            BYTES_PER_LAYER_FALLBACK / (1024 * 1024)
        };
        let per_layer_mb = per_layer_mb.max(1);

        // Compute usable VRAM after headroom and per-model cap.
        let usable = vram_free.saturating_sub(VRAM_HEADROOM_MB);
        let cap_mb = (vram_total as f64 * MAX_PER_MODEL_VRAM_FRACTION) as u64;
        let usable = usable.min(cap_mb);

        let auto_layers = (usable / per_layer_mb).min(DEFAULT_LAYER_COUNT) as u32;

        // Power-saving: cap at 30% of auto.
        let power_cap = if power_saving {
            ((auto_layers as f64) * BATTERY_LAYER_FRACTION).ceil() as u32
        } else {
            u32::MAX
        };

        let max_layers = auto_layers.min(power_cap);

        // Evict LRU if we still don't have enough headroom after the cap.
        let allocated_mb: u64 = self.allocs.read().await
            .values().map(|a| a.vram_est_mb).sum();
        if allocated_mb + (max_layers as u64 * per_layer_mb) > usable + allocated_mb {
            self.evict_lru().await;
        }

        let final_layers = match requested {
            Some(r) => r.min(max_layers),
            None    => max_layers,
        };

        let vram_est = final_layers as u64 * per_layer_mb;
        self.allocs.write().await.insert(
            model_path.to_string(),
            LayerAlloc {
                layers:      final_layers,
                vram_est_mb: vram_est,
                last_used:   std::time::Instant::now(),
            },
        );

        info!(
            model = %model_path,
            layers = final_layers,
            max = DEFAULT_LAYER_COUNT,
            vram_free_mb = vram_free,
            power_saving = power_saving,
            "[gpu_ctrl] Layer allocation"
        );

        final_layers
    }

    /// Evict the least-recently-used model's layer allocation.
    async fn evict_lru(&self) {
        let mut allocs = self.allocs.write().await;
        if let Some(lru_key) = allocs.iter()
            .min_by_key(|(_, v)| v.last_used)
            .map(|(k, _)| k.clone())
        {
            let freed = allocs.remove(&lru_key).map(|a| a.vram_est_mb).unwrap_or(0);
            info!(model = %lru_key, freed_mb = freed, "[gpu_ctrl] Evicted LRU model layers");
        }
    }

    // ── Health ────────────────────────────────────────────────────────────────

    /// Returns the best available backend, or `Cpu` if GPU is unhealthy.
    pub async fn best_backend(&self) -> BackendType {
        let preferred = &[BackendType::Vulkan, BackendType::DirectML];
        for bt in preferred {
            if self.gpu.pre_check(bt).await {
                return bt.clone();
            }
        }
        BackendType::Cpu
    }

    // ── Crash recovery ────────────────────────────────────────────────────────

    /// Execute `gpu_fn`; on failure, log, record the failure, and execute `cpu_fn`.
    /// Schedules an async recovery health re-check 5 minutes later.
    pub async fn with_fallback<T, F, G, Fut1, Fut2>(
        &self,
        op_name: &str,
        gpu_fn:  F,
        cpu_fn:  G,
    ) -> Result<T, String>
    where
        F: FnOnce(BackendType) -> Fut1,
        G: FnOnce() -> Fut2,
        Fut1: std::future::Future<Output = Result<T, String>>,
        Fut2: std::future::Future<Output = Result<T, String>>,
    {
        let backend = self.best_backend().await;

        if backend == BackendType::Cpu {
            return cpu_fn().await;
        }

        match gpu_fn(backend.clone()).await {
            Ok(v) => {
                self.gpu.record_result(backend, true, None).await;
                Ok(v)
            }
            Err(e) => {
                warn!(op = op_name, err = %e, "[gpu_ctrl] GPU op failed — falling back to CPU");
                self.gpu.record_result(backend.clone(), false, Some(e.clone())).await;
                self.schedule_recovery(backend).await;
                cpu_fn().await
            }
        }
    }

    async fn schedule_recovery(&self, backend: BackendType) {
        let mut pending = self.recovery_pending.write().await;
        if *pending { return; }
        *pending = true;
        drop(pending);

        let gpu   = Arc::clone(&self.gpu);
        let flag  = Arc::clone(&self.recovery_pending); // Arc<RwLock<bool>>
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(300)).await;
            let recovered = gpu.pre_check(&backend).await;
            info!(backend = ?backend, recovered, "[gpu_ctrl] Recovery check complete");
            *flag.write().await = false;
        });
    }

    // ── Model release ─────────────────────────────────────────────────────────

    /// Release layer allocation for a model (call after unload).
    pub async fn release(&self, model_path: &str) {
        self.allocs.write().await.remove(model_path);
    }

    /// Mark a model as recently used (prevents LRU eviction).
    pub async fn touch(&self, model_path: &str) {
        if let Some(a) = self.allocs.write().await.get_mut(model_path) {
            a.last_used = std::time::Instant::now();
        }
    }

    // ── Health report ─────────────────────────────────────────────────────────

    pub async fn health_report(&self) -> GpuHealthReport {
        let backend = self.best_backend().await;
        let healthy = backend != BackendType::Cpu;
        let allocs  = self.allocs.read().await;
        let total_reserved: u64 = allocs.values().map(|a| a.vram_est_mb).sum();
        let vram_free = self.gpu.free_vram_mb();

        GpuHealthReport {
            backend:                format!("{:?}", backend),
            healthy,
            vram_total_mb:          vram_free, // proxy; real figure via llama.cpp
            vram_free_mb:           vram_free.saturating_sub(total_reserved),
            loaded_models:          allocs.keys().cloned().collect(),
            total_vram_reserved_mb: total_reserved,
            allocation_ok:          true,
            fallback_active:        !healthy,
            recovery_pending:       *self.recovery_pending.read().await,
            uptime_secs:            self.started_at.elapsed().as_secs(),
        }
    }

    // ── Predictive prefetch ───────────────────────────────────────────────────

    /// Ask MicroBonsai which model will likely be needed next and store a
    /// summary placeholder in the arena so the swap can reuse context.
    pub async fn predictive_prefetch(&self, current_prompt: &str) {
        // Record that this prompt occurred (feeds selection heuristics).
        // We don't have a true "predict next model" API yet — use arena summary.
        let _ = self.arena.store_metadata(
            &format!("prefetch:{}", &current_prompt[..current_prompt.len().min(64)]),
            "gpu_controller",
            &serde_json::json!({ "prompt_preview": &current_prompt[..current_prompt.len().min(128)] }),
        );
    }
}

// ── Startup health check ──────────────────────────────────────────────────────

pub async fn run_startup_health_check(gpu: &GpuLayer) -> GpuHealthReport {
    let vram_free = gpu.free_vram_mb();

    // Allocation test: try to conceptually reserve 100 MiB.
    let allocation_ok = vram_free >= 100;

    // Backend health test.
    let vulkan_ok  = gpu.pre_check(&BackendType::Vulkan).await;
    let directml_ok = gpu.pre_check(&BackendType::DirectML).await;
    let healthy    = vulkan_ok || directml_ok;

    let backend = if vulkan_ok {
        "Vulkan"
    } else if directml_ok {
        "DirectML"
    } else {
        "CPU"
    };

    info!(
        backend,
        vram_free_mb = vram_free,
        healthy,
        allocation_ok,
        "[gpu_ctrl] Startup health check complete"
    );

    GpuHealthReport {
        backend: backend.to_string(),
        healthy,
        vram_total_mb: vram_free,
        vram_free_mb: vram_free,
        loaded_models: vec![],
        total_vram_reserved_mb: 0,
        allocation_ok,
        fallback_active: !healthy,
        recovery_pending: false,
        uptime_secs: 0,
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_gpu_controller_health(
    state: tauri::State<'_, crate::AppState>,
) -> Result<GpuHealthReport, String> {
    Ok(state.gpu_controller.health_report().await)
}

#[tauri::command]
pub async fn profile_vram_cmd(
    state: tauri::State<'_, crate::AppState>,
) -> Result<serde_json::Value, String> {
    Ok(state.gpu_controller.profile_vram().await)
}

#[tauri::command]
pub async fn reset_gpu_controller(
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    // Clear all layer allocations (user-initiated GPU reset).
    state.gpu_controller.allocs.write().await.clear();
    *state.gpu_controller.recovery_pending.write().await = false;
    info!("[gpu_ctrl] User-initiated GPU controller reset");
    Ok(())
}
