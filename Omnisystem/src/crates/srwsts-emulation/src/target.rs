//! Emulation target ISA definitions

use serde::{Deserialize, Serialize};

/// Supported emulation targets (ISAs)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmulationTarget {
    /// x86-64 / x64 architecture
    X86_64,
    /// ARM 64-bit (ARMv8)
    ARMv8,
    /// RISC-V 64-bit
    RiscV64,
}

impl std::fmt::Display for EmulationTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::X86_64 => write!(f, "x86-64"),
            Self::ARMv8 => write!(f, "ARMv8"),
            Self::RiscV64 => write!(f, "RISC-V 64"),
        }
    }
}

impl Default for EmulationTarget {
    fn default() -> Self {
        Self::X86_64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emulation_target_display() {
        assert_eq!(EmulationTarget::X86_64.to_string(), "x86-64");
        assert_eq!(EmulationTarget::ARMv8.to_string(), "ARMv8");
        assert_eq!(EmulationTarget::RiscV64.to_string(), "RISC-V 64");
    }

    #[test]
    fn test_emulation_target_default() {
        assert_eq!(EmulationTarget::default(), EmulationTarget::X86_64);
    }
}
