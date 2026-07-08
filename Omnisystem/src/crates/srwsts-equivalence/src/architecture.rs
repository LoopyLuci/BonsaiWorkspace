//! Architecture definitions and specifications for equivalence testing
//!
//! Supports x86_64, ARMv8, and RISC-V with detailed microarchitecture variants.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Architecture target enum
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArchitectureTarget {
    /// x86_64 architecture with specific variant
    X86_64(ArchVariant),
    /// ARMv8 architecture with specific variant
    ARMv8(ArchVariant),
    /// RISC-V 64-bit architecture
    RiscV64(RiscVVariant),
    /// Emulated version of architecture for testing
    Emulated(Box<ArchitectureTarget>),
}

impl fmt::Display for ArchitectureTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::X86_64(v) => write!(f, "x86_64-{}", v),
            Self::ARMv8(v) => write!(f, "armv8-{}", v),
            Self::RiscV64(v) => write!(f, "riscv64-{}", v),
            Self::Emulated(arch) => write!(f, "emulated-{}", arch),
        }
    }
}

impl ArchitectureTarget {
    /// Get the base architecture string
    pub fn base_arch(&self) -> &'static str {
        match self {
            Self::X86_64(_) => "x86_64",
            Self::ARMv8(_) => "armv8",
            Self::RiscV64(_) => "riscv64",
            Self::Emulated(arch) => arch.base_arch(),
        }
    }

    /// Check if this is an emulated architecture
    pub fn is_emulated(&self) -> bool {
        matches!(self, Self::Emulated(_))
    }

    /// Get native architecture (if emulated, unwrap to base)
    pub fn native_arch(&self) -> Self {
        match self {
            Self::Emulated(arch) => arch.native_arch(),
            other => other.clone(),
        }
    }

    /// Get CPU frequency in MHz for this variant
    pub fn cpu_frequency_mhz(&self) -> u32 {
        match self {
            Self::X86_64(ArchVariant::Skylake) => 3600,
            Self::X86_64(ArchVariant::EPYC) => 3400,
            Self::X86_64(ArchVariant::Xeon) => 3500,
            Self::X86_64(ArchVariant::CortexA72) => 1500,
            Self::X86_64(ArchVariant::CortexA76) => 1500,
            Self::ARMv8(ArchVariant::Skylake) => 3600,
            Self::ARMv8(ArchVariant::EPYC) => 3400,
            Self::ARMv8(ArchVariant::Xeon) => 3500,
            Self::ARMv8(ArchVariant::CortexA72) => 1500,
            Self::ARMv8(ArchVariant::CortexA76) => 2800,
            Self::RiscV64(RiscVVariant::Base) => 1000,
            Self::RiscV64(RiscVVariant::WithVectorExt) => 1000,
            Self::Emulated(arch) => arch.cpu_frequency_mhz(),
        }
    }

    /// Check if architecture supports SIMD
    pub fn supports_simd(&self) -> bool {
        match self {
            Self::X86_64(_) => true,
            Self::ARMv8(_) => true,
            Self::RiscV64(RiscVVariant::WithVectorExt) => true,
            Self::RiscV64(RiscVVariant::Base) => false,
            Self::Emulated(arch) => arch.supports_simd(),
        }
    }

    /// Get cache hierarchy sizes in bytes
    pub fn cache_sizes(&self) -> (usize, usize, usize) {
        match self {
            Self::X86_64(ArchVariant::Skylake) => (32 * 1024, 256 * 1024, 8 * 1024 * 1024),
            Self::X86_64(ArchVariant::EPYC) => (32 * 1024, 512 * 1024, 16 * 1024 * 1024),
            Self::X86_64(ArchVariant::Xeon) => (32 * 1024, 256 * 1024, 11 * 1024 * 1024),
            Self::X86_64(ArchVariant::CortexA72) => (32 * 1024, 128 * 1024, 0),
            Self::X86_64(ArchVariant::CortexA76) => (64 * 1024, 256 * 1024, 0),
            Self::ARMv8(ArchVariant::CortexA72) => (32 * 1024, 128 * 1024, 0), // No L3
            Self::ARMv8(ArchVariant::CortexA76) => (64 * 1024, 256 * 1024, 0),
            Self::ARMv8(ArchVariant::Skylake) => (32 * 1024, 256 * 1024, 8 * 1024 * 1024),
            Self::ARMv8(ArchVariant::EPYC) => (32 * 1024, 512 * 1024, 16 * 1024 * 1024),
            Self::ARMv8(ArchVariant::Xeon) => (32 * 1024, 256 * 1024, 11 * 1024 * 1024),
            Self::RiscV64(_) => (16 * 1024, 128 * 1024, 0),
            Self::Emulated(arch) => arch.cache_sizes(),
        }
    }

    /// Get number of cores for this architecture
    pub fn core_count(&self) -> usize {
        match self {
            Self::X86_64(ArchVariant::Skylake) => 8,
            Self::X86_64(ArchVariant::EPYC) => 64,
            Self::X86_64(ArchVariant::Xeon) => 24,
            Self::X86_64(ArchVariant::CortexA72) => 4,
            Self::X86_64(ArchVariant::CortexA76) => 8,
            Self::ARMv8(ArchVariant::CortexA72) => 4,
            Self::ARMv8(ArchVariant::CortexA76) => 8,
            Self::ARMv8(ArchVariant::Skylake) => 8,
            Self::ARMv8(ArchVariant::EPYC) => 64,
            Self::ARMv8(ArchVariant::Xeon) => 24,
            Self::RiscV64(_) => 4,
            Self::Emulated(arch) => arch.core_count(),
        }
    }

    /// Get memory latency profile
    pub fn memory_latency_ns(&self) -> MemoryLatency {
        match self {
            Self::X86_64(_) => MemoryLatency {
                l1_hit_ns: 4,
                l2_hit_ns: 12,
                l3_hit_ns: 40,
                main_memory_ns: 100,
            },
            Self::ARMv8(_) => MemoryLatency {
                l1_hit_ns: 4,
                l2_hit_ns: 11,
                l3_hit_ns: 0, // No L3
                main_memory_ns: 100,
            },
            Self::RiscV64(_) => MemoryLatency {
                l1_hit_ns: 3,
                l2_hit_ns: 10,
                l3_hit_ns: 0,
                main_memory_ns: 100,
            },
            Self::Emulated(arch) => arch.memory_latency_ns(),
        }
    }

    /// Get NUMA node count
    pub fn numa_nodes(&self) -> usize {
        match self {
            Self::X86_64(ArchVariant::EPYC) => 2,
            Self::X86_64(_) => 1,
            Self::ARMv8(_) => 1,
            Self::RiscV64(_) => 1,
            Self::Emulated(arch) => arch.numa_nodes(),
        }
    }
}

/// x86_64 architecture variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArchVariant {
    /// Intel Skylake
    Skylake,
    /// AMD EPYC
    EPYC,
    /// Intel Xeon
    Xeon,
    /// ARM Cortex-A72
    CortexA72,
    /// ARM Cortex-A76
    CortexA76,
}

impl fmt::Display for ArchVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Skylake => write!(f, "skylake"),
            Self::EPYC => write!(f, "epyc"),
            Self::Xeon => write!(f, "xeon"),
            Self::CortexA72 => write!(f, "cortex-a72"),
            Self::CortexA76 => write!(f, "cortex-a76"),
        }
    }
}

/// RISC-V architecture variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RiscVVariant {
    /// Base RV64 without vector extensions
    Base,
    /// RV64 with vector extension support
    WithVectorExt,
}

impl fmt::Display for RiscVVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Base => write!(f, "base"),
            Self::WithVectorExt => write!(f, "with-vector"),
        }
    }
}

/// Memory latency profile for an architecture
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MemoryLatency {
    /// L1 cache hit latency in nanoseconds
    pub l1_hit_ns: u32,
    /// L2 cache hit latency in nanoseconds
    pub l2_hit_ns: u32,
    /// L3 cache hit latency in nanoseconds (0 if no L3)
    pub l3_hit_ns: u32,
    /// Main memory access latency in nanoseconds
    pub main_memory_ns: u32,
}

impl MemoryLatency {
    /// Get average latency across cache hierarchy
    pub fn average_latency_ns(&self) -> u32 {
        (self.l1_hit_ns + self.l2_hit_ns + self.l3_hit_ns + self.main_memory_ns) / 4
    }
}

/// Architecture feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureFeatures {
    pub sse: bool,
    pub sse2: bool,
    pub sse3: bool,
    pub sse41: bool,
    pub sse42: bool,
    pub avx: bool,
    pub avx2: bool,
    pub avx512: bool,
    pub neon: bool,
    pub rv_vector: bool,
    pub aes_ni: bool,
    pub clmul: bool,
    pub rdrand: bool,
    pub rdseed: bool,
}

impl ArchitectureFeatures {
    /// Get features for a specific architecture
    pub fn for_architecture(arch: &ArchitectureTarget) -> Self {
        match arch {
            ArchitectureTarget::X86_64(ArchVariant::Skylake) => Self {
                sse: true,
                sse2: true,
                sse3: true,
                sse41: true,
                sse42: true,
                avx: true,
                avx2: true,
                avx512: false,
                neon: false,
                rv_vector: false,
                aes_ni: true,
                clmul: true,
                rdrand: true,
                rdseed: false,
            },
            ArchitectureTarget::X86_64(ArchVariant::EPYC) => Self {
                sse: true,
                sse2: true,
                sse3: true,
                sse41: true,
                sse42: true,
                avx: true,
                avx2: true,
                avx512: true,
                neon: false,
                rv_vector: false,
                aes_ni: true,
                clmul: true,
                rdrand: true,
                rdseed: true,
            },
            ArchitectureTarget::X86_64(ArchVariant::Xeon) => Self {
                sse: true,
                sse2: true,
                sse3: true,
                sse41: true,
                sse42: true,
                avx: true,
                avx2: true,
                avx512: true,
                neon: false,
                rv_vector: false,
                aes_ni: true,
                clmul: true,
                rdrand: true,
                rdseed: true,
            },
            ArchitectureTarget::X86_64(ArchVariant::CortexA72) => Self {
                sse: false,
                sse2: false,
                sse3: false,
                sse41: false,
                sse42: false,
                avx: false,
                avx2: false,
                avx512: false,
                neon: false,
                rv_vector: false,
                aes_ni: false,
                clmul: false,
                rdrand: false,
                rdseed: false,
            },
            ArchitectureTarget::X86_64(ArchVariant::CortexA76) => Self {
                sse: false,
                sse2: false,
                sse3: false,
                sse41: false,
                sse42: false,
                avx: false,
                avx2: false,
                avx512: false,
                neon: false,
                rv_vector: false,
                aes_ni: false,
                clmul: false,
                rdrand: false,
                rdseed: false,
            },
            ArchitectureTarget::ARMv8(ArchVariant::Skylake) => Self {
                sse: true,
                sse2: true,
                sse3: true,
                sse41: true,
                sse42: true,
                avx: true,
                avx2: true,
                avx512: false,
                neon: false,
                rv_vector: false,
                aes_ni: true,
                clmul: true,
                rdrand: true,
                rdseed: false,
            },
            ArchitectureTarget::ARMv8(ArchVariant::EPYC) => Self {
                sse: true,
                sse2: true,
                sse3: true,
                sse41: true,
                sse42: true,
                avx: true,
                avx2: true,
                avx512: true,
                neon: false,
                rv_vector: false,
                aes_ni: true,
                clmul: true,
                rdrand: true,
                rdseed: true,
            },
            ArchitectureTarget::ARMv8(ArchVariant::Xeon) => Self {
                sse: true,
                sse2: true,
                sse3: true,
                sse41: true,
                sse42: true,
                avx: true,
                avx2: true,
                avx512: true,
                neon: false,
                rv_vector: false,
                aes_ni: true,
                clmul: true,
                rdrand: true,
                rdseed: true,
            },
            ArchitectureTarget::ARMv8(ArchVariant::CortexA72) => Self {
                sse: false,
                sse2: false,
                sse3: false,
                sse41: false,
                sse42: false,
                avx: false,
                avx2: false,
                avx512: false,
                neon: true,
                rv_vector: false,
                aes_ni: false,
                clmul: false,
                rdrand: false,
                rdseed: false,
            },
            ArchitectureTarget::ARMv8(ArchVariant::CortexA76) => Self {
                sse: false,
                sse2: false,
                sse3: false,
                sse41: false,
                sse42: false,
                avx: false,
                avx2: false,
                avx512: false,
                neon: true,
                rv_vector: false,
                aes_ni: false,
                clmul: false,
                rdrand: false,
                rdseed: false,
            },
            ArchitectureTarget::RiscV64(RiscVVariant::WithVectorExt) => Self {
                sse: false,
                sse2: false,
                sse3: false,
                sse41: false,
                sse42: false,
                avx: false,
                avx2: false,
                avx512: false,
                neon: false,
                rv_vector: true,
                aes_ni: false,
                clmul: false,
                rdrand: false,
                rdseed: false,
            },
            ArchitectureTarget::RiscV64(RiscVVariant::Base) => Self {
                sse: false,
                sse2: false,
                sse3: false,
                sse41: false,
                sse42: false,
                avx: false,
                avx2: false,
                avx512: false,
                neon: false,
                rv_vector: false,
                aes_ni: false,
                clmul: false,
                rdrand: false,
                rdseed: false,
            },
            ArchitectureTarget::Emulated(arch) => Self::for_architecture(arch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_architecture_display() {
        let arch = ArchitectureTarget::X86_64(ArchVariant::Skylake);
        assert_eq!(arch.to_string(), "x86_64-skylake");
    }

    #[test]
    fn test_architecture_base_arch() {
        let arch = ArchitectureTarget::ARMv8(ArchVariant::CortexA76);
        assert_eq!(arch.base_arch(), "armv8");
    }

    #[test]
    fn test_cache_sizes() {
        let arch = ArchitectureTarget::X86_64(ArchVariant::EPYC);
        let (l1, l2, l3) = arch.cache_sizes();
        assert_eq!(l1, 32 * 1024);
        assert_eq!(l3, 16 * 1024 * 1024);
    }

    #[test]
    fn test_simd_support() {
        assert!(ArchitectureTarget::X86_64(ArchVariant::Skylake).supports_simd());
        assert!(ArchitectureTarget::ARMv8(ArchVariant::CortexA76).supports_simd());
        assert!(!ArchitectureTarget::RiscV64(RiscVVariant::Base).supports_simd());
    }

    #[test]
    fn test_features_for_architecture() {
        let features = ArchitectureFeatures::for_architecture(
            &ArchitectureTarget::X86_64(ArchVariant::Skylake),
        );
        assert!(features.avx2);
        assert!(!features.avx512);
    }

    #[test]
    fn test_emulated_architecture() {
        let emulated =
            ArchitectureTarget::Emulated(Box::new(ArchitectureTarget::X86_64(ArchVariant::Xeon)));
        assert!(emulated.is_emulated());
        assert_eq!(emulated.base_arch(), "x86_64");
    }

    #[test]
    fn test_numa_nodes() {
        let epyc = ArchitectureTarget::X86_64(ArchVariant::EPYC);
        assert_eq!(epyc.numa_nodes(), 2);

        let skylake = ArchitectureTarget::X86_64(ArchVariant::Skylake);
        assert_eq!(skylake.numa_nodes(), 1);
    }

    #[test]
    fn test_memory_latency() {
        let arch = ArchitectureTarget::X86_64(ArchVariant::Skylake);
        let latency = arch.memory_latency_ns();
        assert_eq!(latency.l1_hit_ns, 4);
        assert_eq!(latency.main_memory_ns, 100);
    }
}
