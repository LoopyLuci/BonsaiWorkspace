use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, warn};

// ── Feature-gated native backend ─────────────────────────────────────────────

#[cfg(feature = "native-gpu")]
use bonsai_native::HybridEngine;

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStatusDto {
    pub total_vram_mb: u64,
    pub free_vram_mb: u64,
    pub total_ram_mb: u64,
    pub free_ram_mb: u64,
    pub recommended_gpu_layers: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeEngineStatus {
    pub enabled: bool,
    pub model_loaded: bool,
    pub current_model_path: Option<String>,
    pub n_gpu_layers: i32,
    pub memory: Option<MemoryStatusDto>,
}

// ── Internal state ────────────────────────────────────────────────────────────

struct Inner {
    enabled: bool,
    current_model_path: Option<String>,
    n_gpu_layers: i32,
    #[cfg(feature = "native-gpu")]
    engine: HybridEngine,
}

pub struct HybridEngineState {
    inner: RwLock<Inner>,
}

impl HybridEngineState {
    pub fn new() -> Self {
        let enabled = crate::features::FeatureFlags::is_enabled("hybrid_engine_enabled");

        if enabled {
            info!("[hybrid_engine] feature flag enabled — native GPU inference active");
        } else {
            info!("[hybrid_engine] feature flag disabled — using HTTP inference only");
        }

        Self {
            inner: RwLock::new(Inner {
                enabled,
                current_model_path: None,
                n_gpu_layers: 0,
                #[cfg(feature = "native-gpu")]
                engine: HybridEngine::new(),
            }),
        }
    }

    pub async fn is_enabled(&self) -> bool {
        self.inner.read().await.enabled
    }

    /// Load a GGUF model. No-op if feature flag is off or native-gpu feature not compiled.
    pub async fn load_model(&self, path: &str, n_gpu_layers: i32) -> Result<(), String> {
        let mut guard = self.inner.write().await;
        if !guard.enabled {
            return Err("hybrid_engine_enabled feature flag is off".into());
        }
        #[cfg(feature = "native-gpu")]
        {
            guard.engine.load(path, n_gpu_layers).await
                .map_err(|e| e.to_string())?;
            guard.current_model_path = Some(path.to_string());
            guard.n_gpu_layers = n_gpu_layers;
            info!("[hybrid_engine] model loaded: {} ({} GPU layers)", path, n_gpu_layers);
            Ok(())
        }
        #[cfg(not(feature = "native-gpu"))]
        {
            warn!("[hybrid_engine] compiled without native-gpu feature — load_model is a no-op");
            Err("bonsai compiled without native-gpu feature".into())
        }
    }

    /// Apply a LoRA adapter. No-op if native-gpu feature not compiled.
    pub async fn apply_lora(&self, lora_path: &str, scale: f32) -> Result<(), String> {
        let guard = self.inner.read().await;
        if !guard.enabled {
            return Err("hybrid_engine_enabled feature flag is off".into());
        }
        #[cfg(feature = "native-gpu")]
        {
            guard.engine.apply_lora(lora_path, scale).await
                .map_err(|e| e.to_string())
        }
        #[cfg(not(feature = "native-gpu"))]
        {
            Err("bonsai compiled without native-gpu feature".into())
        }
    }

    pub async fn status(&self) -> NativeEngineStatus {
        let guard = self.inner.read().await;

        #[cfg(feature = "native-gpu")]
        let (model_loaded, memory) = {
            let loaded = guard.engine.is_loaded().await;
            let mem = if loaded {
                let ms = guard.engine.memory_status().await;
                Some(MemoryStatusDto {
                    total_vram_mb: ms.total_vram_mb,
                    free_vram_mb: ms.free_vram_mb,
                    total_ram_mb: ms.total_ram_mb,
                    free_ram_mb: ms.free_ram_mb,
                    recommended_gpu_layers: ms.recommended_gpu_layers,
                })
            } else {
                None
            };
            (loaded, mem)
        };

        #[cfg(not(feature = "native-gpu"))]
        let (model_loaded, memory) = (false, None);

        NativeEngineStatus {
            enabled: guard.enabled,
            model_loaded,
            current_model_path: guard.current_model_path.clone(),
            n_gpu_layers: guard.n_gpu_layers,
            memory,
        }
    }
}

impl Default for HybridEngineState {
    fn default() -> Self {
        Self::new()
    }
}
