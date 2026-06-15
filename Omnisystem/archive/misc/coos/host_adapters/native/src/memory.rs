use serde::Serialize;
use sysinfo::System;

#[derive(Debug, Clone, Serialize)]
pub struct MemoryStatus {
    pub total_vram_mb: u64,
    pub free_vram_mb: u64,
    pub total_ram_mb: u64,
    pub free_ram_mb: u64,
    pub recommended_gpu_layers: u32,
}

pub struct MemoryManager {
    sys: System,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            sys: System::new_all(),
        }
    }

    pub fn refresh(&mut self) {
        self.sys.refresh_all();
    }

    /// Heuristic: available VRAM / 350 MB per layer (gemma-4-31B Q2 baseline).
    /// Clamped to [0, 80] — llama.cpp allows up to model layer count.
    pub fn recommend_gpu_layers(&self, model_size_gb: f32) -> u32 {
        // Without direct VRAM query we use RAM headroom as proxy; callers
        // that have real VRAM info should override this.
        let free_ram_mb = self.sys.free_memory() / 1024 / 1024;
        let mb_per_layer = (model_size_gb * 1024.0 / 80.0) as u64; // rough
        if mb_per_layer == 0 {
            return 0;
        }
        ((free_ram_mb / mb_per_layer) as u32).min(80)
    }

    pub fn can_load(&self, required_mb: u64) -> bool {
        let free_ram_mb = self.sys.free_memory() / 1024 / 1024;
        free_ram_mb >= required_mb
    }

    pub fn status(&self) -> MemoryStatus {
        let total_ram_mb = self.sys.total_memory() / 1024 / 1024;
        let free_ram_mb = self.sys.free_memory() / 1024 / 1024;
        MemoryStatus {
            total_vram_mb: 0, // populated by HybridEngine after vulkan query
            free_vram_mb: 0,
            total_ram_mb,
            free_ram_mb,
            recommended_gpu_layers: self.recommend_gpu_layers(31.0),
        }
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}
