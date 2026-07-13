use anyhow::Result;
use std::path::Path;

pub struct GpuManager;

impl GpuManager {
    pub fn new() -> Self {
        Self
    }

    pub fn detect_optimal_layers(&self, _model_path: &Path) -> Result<u32> {
        let vram_bytes = self.detect_vram_bytes();

        if vram_bytes == 0 {
            tracing::info!("No GPU detected, using CPU inference");
            return Ok(0);
        }

        // Assume 4 bytes per parameter
        let model_size_f32 = 7_000_000_000 * 4; // ~7B model = 28GB in float32
        let max_layers = 80;
        let vram_for_layers = (vram_bytes as f64 * 0.8) as u64;

        if model_size_f32 <= vram_for_layers {
            tracing::info!("Model fits in VRAM, offloading all {} layers to GPU", max_layers);
            return Ok(max_layers);
        }

        let layer_size = model_size_f32 / max_layers as u64;
        let layers = ((vram_for_layers / layer_size).min(max_layers as u64)) as u32;
        tracing::info!("Offloading {} out of {} layers to GPU", layers, max_layers);
        Ok(layers)
    }

    fn detect_vram_bytes(&self) -> u64 {
        // Try nvidia-smi on Windows/Linux
        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
            .output()
        {
            if let Ok(vram_mb) = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<u64>()
            {
                tracing::info!("Detected NVIDIA GPU with {} MB VRAM", vram_mb);
                return vram_mb * 1024 * 1024;
            }
        }

        // Try Metal on macOS
        if let Ok(output) = std::process::Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
        {
            if let Ok(ram) = String::from_utf8_lossy(&output.stdout).trim().parse::<u64>() {
                // Allocate half of system RAM for Metal GPU
                let metal_vram = ram / 2;
                tracing::info!("Detected macOS, allocating {} bytes for Metal GPU", metal_vram);
                return metal_vram;
            }
        }

        tracing::warn!("Could not detect GPU; will use CPU inference");
        0
    }
}

impl Default for GpuManager {
    fn default() -> Self {
        Self::new()
    }
}
