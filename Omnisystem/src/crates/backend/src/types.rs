use serde::{Serialize, Deserialize};
use std::fmt;

/// Hardware profile describing available compute and memory resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub cpu: CpuProfile,
    pub gpus: Vec<GpuProfile>,
    pub memory: MemoryProfile,
}

/// CPU information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuProfile {
    pub vendor: String,
    pub model: String,
    pub physical_cores: u32,
    pub logical_cores: u32,
    pub frequency_mhz: u64,
    pub cache_l3_mb: u64,
    pub simd_features: Vec<String>,  // ["avx2", "avx512f", "neon"]
}

/// GPU information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuProfile {
    pub index: u32,
    pub name: String,
    pub vram_bytes: u64,
    pub compute_units: u32,
    pub backend: GpuBackend,
    pub supports_fp16: bool,
    pub supports_bf16: bool,
    pub supports_int8: bool,
}

/// Supported GPU backends.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GpuBackend {
    Cuda,
    Rocm,
    Metal,
    Vulkan,
    DirectML,
    None,
}

impl fmt::Display for GpuBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cuda => write!(f, "CUDA"),
            Self::Rocm => write!(f, "ROCm"),
            Self::Metal => write!(f, "Metal"),
            Self::Vulkan => write!(f, "Vulkan"),
            Self::DirectML => write!(f, "DirectML"),
            Self::None => write!(f, "None"),
        }
    }
}

/// Memory information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryProfile {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

/// Requirements for resource allocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequirements {
    pub task_type: TaskType,
    pub estimated_memory_bytes: u64,
    pub min_compute_units: u32,
    pub precision: Precision,
    pub allow_fallback: bool,
}

/// Type of computational task.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskType {
    Inference,
    Training,
    Embedding,
    Encoding,
    Other,
}

/// Numerical precision for computations.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Precision {
    FP32,
    FP16,
    BF16,
    INT8,
    INT4,
    Auto,
}

impl fmt::Display for Precision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FP32 => write!(f, "FP32"),
            Self::FP16 => write!(f, "FP16"),
            Self::BF16 => write!(f, "BF16"),
            Self::INT8 => write!(f, "INT8"),
            Self::INT4 => write!(f, "INT4"),
            Self::Auto => write!(f, "Auto"),
        }
    }
}

/// Result of resource allocation.
#[derive(Debug, Clone)]
pub struct DeviceAllocation {
    pub devices: Vec<Device>,
    pub batch_size: u32,
    pub use_cpu_fallback: bool,
    pub precision: Precision,
}

/// Allocated device.
#[derive(Debug, Clone)]
pub struct Device {
    pub device_type: DeviceType,
    pub index: u32,
    pub memory_allocated_bytes: u64,
}

/// Type of compute device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Cpu,
    Gpu,
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cpu => write!(f, "CPU"),
            Self::Gpu => write!(f, "GPU"),
        }
    }
}
