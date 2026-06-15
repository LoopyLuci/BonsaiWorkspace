use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GpuDevice {
    pub device_id: u32,
    pub name: String,
    pub memory_mb: u64,
    pub compute_capability: String,
    pub available: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShaderProgram {
    pub program_id: Uuid,
    pub source: String,
    pub compiled: bool,
    pub optimization_level: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KernelExecution {
    pub kernel_id: Uuid,
    pub grid_size: (u32, u32, u32),
    pub block_size: (u32, u32, u32),
    pub execution_time_us: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimdOperation {
    pub operation_id: Uuid,
    pub vector_width: usize,
    pub element_count: usize,
    pub operation_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GpuMemory {
    pub memory_id: Uuid,
    pub size_bytes: u64,
    pub used_bytes: u64,
    pub allocated: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KernelResult {
    pub kernel_id: Uuid,
    pub device_id: u32,
    pub execution_time_us: u64,
    pub success: bool,
}
