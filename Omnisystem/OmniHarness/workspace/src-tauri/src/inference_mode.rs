use serde::{Deserialize, Serialize};

/// Runtime execution mode for local model inference.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum InferenceMode {
    /// Let the orchestrator choose GPU or CPU based on device capability.
    Auto,
    /// Force CPU-only execution.
    CpuOnly,
    /// Force GPU execution; do not retry in CPU mode on GPU faults.
    GpuOnly,
    /// Prefer GPU with a fixed number of GPU layers.
    Hybrid { gpu_layers: u32 },
}

impl Default for InferenceMode {
    fn default() -> Self {
        // `Hybrid { gpu_layers: N }` returns that fixed N unconditionally from
        // `gpu_layers()` below, bypassing the orchestrator's quant-safety check
        // (some quantizations, e.g. IQ1_S, crash the Vulkan backend outright —
        // see `is_gpu_unsafe_quant` in model_orchestrator.rs) and its VRAM-aware
        // layer estimate entirely. A flat default of 20 layers doesn't know
        // whether that's safe for the model about to load, which is exactly
        // what caused GPU loads to crash regardless of model size or quant.
        // `Auto` defers to the orchestrator's real computation instead; `Hybrid`
        // remains available as an explicit, user-chosen override.
        Self::Auto
    }
}

impl InferenceMode {
    pub fn gpu_layers(&self, gpu_preferred_layers: u32) -> u32 {
        match self {
            Self::CpuOnly => 0,
            Self::GpuOnly => gpu_preferred_layers.max(1),
            Self::Hybrid { gpu_layers } => *gpu_layers,
            Self::Auto => gpu_preferred_layers,
        }
    }

    pub fn allows_cpu_fallback(&self) -> bool {
        !matches!(self, Self::GpuOnly)
    }
}
