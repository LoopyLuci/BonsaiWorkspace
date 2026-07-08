//! Configuration for bonsai-bedf-sanitizers

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SanitizerConfig {
    pub enabled: bool,
    pub enable_asan: bool,
    pub enable_msan: bool,
    pub enable_tsan: bool,
    pub enable_lsan: bool,
    pub timeout_secs: u64,
}

impl Default for SanitizerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enable_asan: true,
            enable_msan: true,
            enable_tsan: true,
            enable_lsan: true,
            timeout_secs: 60,
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
