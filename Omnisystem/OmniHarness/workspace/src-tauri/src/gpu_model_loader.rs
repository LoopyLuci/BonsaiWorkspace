use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

use tracing::info;

use crate::gpu_layer::GpuLayer;

pub struct GpuModelConfig {
    pub model_path: String,
    pub port: u16,
    pub context_size: u32,
    pub force_cpu_fallback: bool,
}

impl GpuModelConfig {
    pub fn new(model_path: &str) -> Self {
        Self {
            model_path: model_path.into(),
            port: 11420,
            context_size: 4096,
            force_cpu_fallback: false,
        }
    }
}

pub struct GpuModelLoader {
    gpu: Arc<GpuLayer>,
}

impl GpuModelLoader {
    pub fn new(gpu: Arc<GpuLayer>) -> Self {
        Self { gpu }
    }

    /// Calculate optimal GPU layers for a model based on available VRAM.
    ///
    /// Uses a 4 GB headroom to cover KV cache, recurrent state buffers, and
    /// MoE compute/activation allocations (empirically needed for Qwen3.5-MoE).
    pub fn calculate_gpu_layers(&self, model_path: &Path) -> u32 {
        let file_size_mb = std::fs::metadata(model_path)
            .map(|m| m.len() / (1024 * 1024))
            .unwrap_or(0);

        let total_layers: u64 = 40;
        let per_layer_mb = (file_size_mb / total_layers).max(1);
        let free_mb = self.gpu.free_vram_mb();
        // 4 GB headroom: 2 GB KV + recurrent state + MoE compute buffers (~553 MB)
        // + fragmentation margin. Verified empirically on 7900 XTX / Qwen3.5-35B-MoE.
        let headroom_mb: u64 = 4096;

        let usable = free_mb.saturating_sub(headroom_mb);
        // Cap at total_layers - 5 for MoE architectures: the fused Gated Delta Net
        // recurrent state + compute buffers need ~600 MB that the weight-only estimate
        // misses. Verified on 7900 XTX with Qwen3.5-35B-MoE (Q6_K, 21 GB): 40 layers
        // fails, 35 layers loads and runs correctly.
        let safe_max = total_layers.saturating_sub(5);
        let layers = (usable / per_layer_mb).min(safe_max) as u32;

        info!(
            free_mb,
            per_layer_mb, layers, total_layers, safe_max, "[gpu_loader] GPU layer calculation"
        );
        layers
    }

    /// Launch llama-server with optimal GPU layers. Returns (PID, gpu_layers_used).
    pub fn launch(&self, config: &GpuModelConfig) -> Result<(u32, u32), String> {
        let model_path = Path::new(&config.model_path);
        if !model_path.exists() {
            return Err(format!("Model not found: {}", config.model_path));
        }

        let gpu_layers = if config.force_cpu_fallback {
            0
        } else {
            self.calculate_gpu_layers(model_path)
        };

        let llama_binary = Self::find_llama_server()?;

        info!(
            gpu_layers,
            model = %config.model_path,
            "[gpu_loader] launching llama-server"
        );

        let child: Child = Command::new(&llama_binary)
            .args([
                "-m",
                &config.model_path,
                "--port",
                &config.port.to_string(),
                "--host",
                "127.0.0.1",
                "--ctx-size",
                &config.context_size.to_string(),
                "--n-gpu-layers",
                &gpu_layers.to_string(),
                "--no-warmup",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start llama-server: {e}"))?;

        let pid = child.id();
        info!(pid, "[gpu_loader] llama-server started");

        Ok((pid, gpu_layers))
    }

    fn find_llama_server() -> Result<String, String> {
        let paths = [
            r"C:\Users\limpi\AppData\Roaming\com.bonsai.workspace\sidecars\llama-server.exe",
            r"C:\Users\limpi\AppData\Local\com.bonsai.workspace\sidecars\llama-server.exe",
        ];
        for path in &paths {
            if Path::new(path).exists() {
                return Ok(path.to_string());
            }
        }
        Err("llama-server not found in sidecars directory".into())
    }
}
