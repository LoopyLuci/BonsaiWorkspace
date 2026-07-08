//! Capability-Based Hardware Access Control
//!
//! Ensures all hardware access is explicit and auditable

use std::fmt;

/// Base capability trait for all hardware access
pub trait Capability: Send + Sync + fmt::Debug {
    fn capability_type(&self) -> CapabilityType;
    fn is_available(&self) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityType {
    Cpu,
    Gpu,
    Tpu,
}

/// CPU capability with specific features
#[derive(Debug, Clone)]
pub struct CpuCapability {
    pub features: Vec<String>,
    pub available: bool,
}

impl CpuCapability {
    pub fn new(features: Vec<String>) -> Self {
        CpuCapability {
            features,
            available: true,
        }
    }

    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature)
    }

    pub fn with_avx2() -> Self {
        CpuCapability::new(vec!["avx2".to_string()])
    }

    pub fn with_avx512() -> Self {
        CpuCapability::new(vec!["avx512f".to_string()])
    }

    pub fn with_neon() -> Self {
        CpuCapability::new(vec!["neon".to_string()])
    }

    pub fn with_sve() -> Self {
        CpuCapability::new(vec!["sve".to_string()])
    }
}

impl Capability for CpuCapability {
    fn capability_type(&self) -> CapabilityType {
        CapabilityType::Cpu
    }

    fn is_available(&self) -> bool {
        self.available
    }
}

/// GPU capability with vendor and compute capability
#[derive(Debug, Clone)]
pub struct GpuCapability {
    pub vendor: String,
    pub compute_capability: String,
    pub memory_mb: usize,
    pub available: bool,
}

impl GpuCapability {
    pub fn new(vendor: String, compute_capability: String, memory_mb: usize) -> Self {
        GpuCapability {
            vendor,
            compute_capability,
            memory_mb,
            available: true,
        }
    }

    pub fn cuda_11_0(memory_mb: usize) -> Self {
        GpuCapability::new("nvidia".to_string(), "11.0".to_string(), memory_mb)
    }

    pub fn cuda_12_0(memory_mb: usize) -> Self {
        GpuCapability::new("nvidia".to_string(), "12.0".to_string(), memory_mb)
    }

    pub fn rocm_50(memory_mb: usize) -> Self {
        GpuCapability::new("amd".to_string(), "5.0".to_string(), memory_mb)
    }

    pub fn has_sufficient_memory(&self, required_mb: usize) -> bool {
        self.memory_mb >= required_mb
    }
}

impl Capability for GpuCapability {
    fn capability_type(&self) -> CapabilityType {
        CapabilityType::Gpu
    }

    fn is_available(&self) -> bool {
        self.available
    }
}

/// TPU capability (future)
#[derive(Debug, Clone)]
pub struct TpuCapability {
    pub model: String,
    pub available: bool,
}

impl Capability for TpuCapability {
    fn capability_type(&self) -> CapabilityType {
        CapabilityType::Tpu
    }

    fn is_available(&self) -> bool {
        self.available
    }
}

/// Capability context for hardware-accelerated operations
pub struct CapabilityContext {
    pub cpu: Option<CpuCapability>,
    pub gpu: Option<GpuCapability>,
    pub tpu: Option<TpuCapability>,
}

impl CapabilityContext {
    pub fn new() -> Self {
        CapabilityContext {
            cpu: Some(CpuCapability::new(vec![])),
            gpu: None,
            tpu: None,
        }
    }

    pub fn with_cpu(mut self, cpu: CpuCapability) -> Self {
        self.cpu = Some(cpu);
        self
    }

    pub fn with_gpu(mut self, gpu: GpuCapability) -> Self {
        self.gpu = Some(gpu);
        self
    }

    pub fn has_gpu(&self) -> bool {
        self.gpu.as_ref().map_or(false, |g| g.is_available())
    }

    pub fn has_cpu(&self) -> bool {
        self.cpu.as_ref().map_or(false, |c| c.is_available())
    }
}

impl Default for CapabilityContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_capability() {
        let cpu = CpuCapability::with_avx2();
        assert!(cpu.has_feature("avx2"));
        assert!(!cpu.has_feature("avx512f"));
        assert_eq!(cpu.capability_type(), CapabilityType::Cpu);
    }

    #[test]
    fn test_gpu_capability() {
        let gpu = GpuCapability::cuda_11_0(4096);
        assert_eq!(gpu.vendor, "nvidia");
        assert!(gpu.has_sufficient_memory(2048));
        assert!(!gpu.has_sufficient_memory(8192));
    }

    #[test]
    fn test_capability_context() {
        let ctx = CapabilityContext::new()
            .with_cpu(CpuCapability::with_avx2())
            .with_gpu(GpuCapability::cuda_11_0(4096));

        assert!(ctx.has_cpu());
        assert!(ctx.has_gpu());
    }
}
