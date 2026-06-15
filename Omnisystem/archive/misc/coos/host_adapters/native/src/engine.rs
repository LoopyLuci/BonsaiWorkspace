use std::ffi::CString;
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::ffi;
use crate::memory::{MemoryManager, MemoryStatus};

// ── LoadedModel ───────────────────────────────────────────────────────────────

pub struct LoadedModel {
    pub model: *mut ffi::LlamaModel,
    pub ctx: *mut ffi::LlamaContext,
    pub n_gpu_layers: i32,
    pub path: String,
}

// SAFETY: llama.cpp model/context pointers are thread-safe for read access
// after construction; mutable decode calls are serialised by the engine mutex.
unsafe impl Send for LoadedModel {}
unsafe impl Sync for LoadedModel {}

impl Drop for LoadedModel {
    fn drop(&mut self) {
        unsafe {
            if !self.ctx.is_null() {
                ffi::llama_free(self.ctx);
            }
            if !self.model.is_null() {
                ffi::llama_free_model(self.model);
            }
        }
    }
}

// ── HybridEngine ──────────────────────────────────────────────────────────────

pub struct HybridEngine {
    model: Arc<RwLock<Option<LoadedModel>>>,
    mem: RwLock<MemoryManager>,
}

impl HybridEngine {
    pub fn new() -> Self {
        unsafe { ffi::llama_backend_init() };
        Self {
            model: Arc::new(RwLock::new(None)),
            mem: RwLock::new(MemoryManager::new()),
        }
    }

    /// Load a GGUF model with the given GPU layer count.
    pub async fn load(&self, model_path: &str, n_gpu_layers: i32) -> Result<()> {
        let path =
            CString::new(model_path).map_err(|_| anyhow!("model path contains null byte"))?;

        let mut mp = unsafe { ffi::llama_model_default_params() };
        mp.n_gpu_layers = n_gpu_layers;
        mp.use_mmap = true;

        info!(
            "loading model {} with {} GPU layers",
            model_path, n_gpu_layers
        );

        let model_ptr = unsafe { ffi::llama_load_model_from_file(path.as_ptr(), mp) };
        if model_ptr.is_null() {
            return Err(anyhow!(
                "llama_load_model_from_file returned null for {}",
                model_path
            ));
        }

        let cp = unsafe { ffi::llama_context_default_params() };
        let ctx_ptr = unsafe { ffi::llama_new_context_with_model(model_ptr, cp) };
        if ctx_ptr.is_null() {
            unsafe { ffi::llama_free_model(model_ptr) };
            return Err(anyhow!("llama_new_context_with_model returned null"));
        }

        let loaded = LoadedModel {
            model: model_ptr,
            ctx: ctx_ptr,
            n_gpu_layers,
            path: model_path.to_string(),
        };

        *self.model.write().await = Some(loaded);
        info!("model loaded successfully");
        Ok(())
    }

    /// Apply a LoRA adapter (additive — multiple adapters can stack).
    pub async fn apply_lora(&self, lora_path: &str, scale: f32) -> Result<()> {
        if !Path::new(lora_path).exists() {
            return Err(anyhow!("LoRA path does not exist: {}", lora_path));
        }

        let guard = self.model.read().await;
        let loaded = guard.as_ref().ok_or_else(|| anyhow!("no model loaded"))?;

        let lora_c =
            CString::new(lora_path).map_err(|_| anyhow!("lora path contains null byte"))?;

        let ret = unsafe {
            ffi::llama_model_apply_lora_from_file(
                loaded.model,
                lora_c.as_ptr(),
                scale,
                std::ptr::null(), // no separate base model needed
                4,
            )
        };

        if ret != 0 {
            return Err(anyhow!("llama_model_apply_lora_from_file failed ({})", ret));
        }

        info!("LoRA adapter applied: {} (scale={})", lora_path, scale);
        Ok(())
    }

    /// Recommended GPU layers for a model of the given size (GB).
    pub async fn auto_gpu_layers(&self, model_size_gb: f32) -> u32 {
        self.mem.read().await.recommend_gpu_layers(model_size_gb)
    }

    /// VRAM usage estimate in MB (returns 0 if model not loaded).
    pub async fn vram_usage_mb(&self) -> u64 {
        let guard = self.model.read().await;
        match guard.as_ref() {
            Some(m) => {
                // Rough estimate: param count * 2 bytes (Q2) / 1024^2
                let params = unsafe { ffi::llama_model_n_params(m.model) };
                (params * 2) / (1024 * 1024)
            }
            None => 0,
        }
    }

    /// VRAM available on GPU 0 (falls back to free RAM if unavailable).
    pub async fn free_vram_mb(&self) -> u64 {
        let mut mem = self.mem.write().await;
        mem.refresh();
        mem.status().free_ram_mb
    }

    pub async fn memory_status(&self) -> MemoryStatus {
        let mut mem = self.mem.write().await;
        mem.refresh();
        let mut status = mem.status();
        status.total_vram_mb = self.vram_usage_mb().await;
        status
    }

    pub async fn is_loaded(&self) -> bool {
        self.model.read().await.is_some()
    }

    pub async fn unload(&self) {
        let mut guard = self.model.write().await;
        if guard.is_some() {
            warn!("unloading native model");
            *guard = None;
        }
    }
}

impl Default for HybridEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for HybridEngine {
    fn drop(&mut self) {
        unsafe { ffi::llama_backend_free() };
    }
}
