//! Configuration for bonsai-bedf-sandbox

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub enabled: bool,
    pub timeout_secs: u64,
    pub max_memory_mb: u64,
    pub enable_seccomp: bool,
    pub enable_landlock: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout_secs: 30,
            max_memory_mb: 512,
            enable_seccomp: true,
            enable_landlock: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { enabled: true }
    }
}
