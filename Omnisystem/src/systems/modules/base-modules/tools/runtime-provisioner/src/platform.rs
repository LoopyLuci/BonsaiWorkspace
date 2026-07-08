//! Platform detection and management

use crate::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
    FreeBSD,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Architecture {
    X86_64,
    X86,
    Arm64,
    Arm32,
    RiscV64,
    RiscV32,
    Wasm32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformInfo {
    pub platform: Platform,
    pub architecture: Architecture,
}

/// Detect current platform and architecture
pub fn detect_platform() -> Result<PlatformInfo> {
    let platform = if cfg!(target_os = "windows") {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOS
    } else if cfg!(target_os = "linux") {
        Platform::Linux
    } else if cfg!(target_os = "freebsd") {
        Platform::FreeBSD
    } else {
        return Err(crate::error::ProvisionerError::PlatformNotSupported(
            std::env::consts::OS.to_string(),
        ));
    };

    let architecture = if cfg!(target_arch = "x86_64") {
        Architecture::X86_64
    } else if cfg!(target_arch = "x86") {
        Architecture::X86
    } else if cfg!(target_arch = "aarch64") {
        Architecture::Arm64
    } else if cfg!(target_arch = "arm") {
        Architecture::Arm32
    } else if cfg!(target_arch = "riscv64") {
        Architecture::RiscV64
    } else if cfg!(target_arch = "riscv32") {
        Architecture::RiscV32
    } else if cfg!(target_arch = "wasm32") {
        Architecture::Wasm32
    } else {
        return Err(crate::error::ProvisionerError::PlatformNotSupported(
            std::env::consts::ARCH.to_string(),
        ));
    };

    Ok(PlatformInfo {
        platform,
        architecture,
    })
}

impl Platform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::Windows => "windows",
            Platform::MacOS => "macos",
            Platform::Linux => "linux",
            Platform::FreeBSD => "freebsd",
        }
    }

    pub fn executable_suffix(&self) -> &'static str {
        match self {
            Platform::Windows => ".exe",
            _ => "",
        }
    }
}

impl Architecture {
    pub fn as_str(&self) -> &'static str {
        match self {
            Architecture::X86_64 => "x86_64",
            Architecture::X86 => "x86",
            Architecture::Arm64 => "arm64",
            Architecture::Arm32 => "arm32",
            Architecture::RiscV64 => "riscv64",
            Architecture::RiscV32 => "riscv32",
            Architecture::Wasm32 => "wasm32",
        }
    }
}

impl PlatformInfo {
    pub fn triple(&self) -> String {
        format!("{}-{}", self.architecture.as_str(), self.platform.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let info = detect_platform();
        assert!(info.is_ok());
    }

    #[test]
    fn test_platform_triple() {
        let info = PlatformInfo {
            platform: Platform::Linux,
            architecture: Architecture::X86_64,
        };
        assert_eq!(info.triple(), "x86_64-linux");
    }
}
