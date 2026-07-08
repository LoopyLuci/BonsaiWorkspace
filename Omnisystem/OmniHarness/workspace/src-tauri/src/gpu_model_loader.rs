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
            // Sidecar llama-server port — distinct from BUDDY_API_PORT even though
            // they historically shared the same excluded-range number (11420).
            port: 47150,
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
    /// Delegates to `model_orchestrator::estimate_safe_gpu_layers`, which is
    /// size-bucketed by transformer depth (24/32/40 layers depending on file
    /// size) and queries real dedicated VRAM via WMI, rather than this
    /// function's former flat `total_layers = 40` assumption combined with
    /// `GpuLayer::free_vram_mb()`'s use of total *system* RAM as a VRAM proxy
    /// — a combination that silently over-offloaded on any model that wasn't
    /// close to the 40-layer/RAM-rich case it was tuned against.
    pub fn calculate_gpu_layers(&self, model_path: &Path) -> u32 {
        crate::model_orchestrator::estimate_safe_gpu_layers(model_path)
    }

    /// Launch llama-server with optimal GPU layers. Returns (PID, gpu_layers_used).
    pub fn launch(&self, config: &GpuModelConfig) -> Result<(u32, u32), String> {
        let model_path = Path::new(&config.model_path);
        if !model_path.exists() {
            return Err(format!("Model not found: {}", config.model_path));
        }

        let quant = crate::model_registry::quant_for_path(model_path);
        let gpu_layers = if config.force_cpu_fallback {
            0
        } else if crate::model_orchestrator::is_gpu_unsafe_quant(&quant) {
            info!(?quant, "[gpu_loader] quant known to crash GPU backend — forcing CPU");
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
        // Was previously hardcoded to a specific developer machine's user
        // profile under the pre-rename bundle id (`com.workspace.workspace`),
        // so it would never resolve on any other machine or account. Derive
        // the real per-user data dirs instead, under the current identifier
        // (`com.omnisystem.workspace`, see `tauri.conf.json`).
        let candidates = [
            dirs::data_dir().map(|d| d.join("com.omnisystem.workspace/sidecars/llama-server.exe")),
            dirs::data_local_dir().map(|d| d.join("com.omnisystem.workspace/sidecars/llama-server.exe")),
        ];
        for path in candidates.into_iter().flatten() {
            if path.exists() {
                return Ok(path.to_string_lossy().into_owned());
            }
        }
        Err("llama-server not found in sidecars directory".into())
    }
}
