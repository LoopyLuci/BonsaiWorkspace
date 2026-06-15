//! Configuration for bonsai-bedf-fuzzing

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FuzzerConfig {
    pub enabled: bool,
    pub max_iterations: u32,
    pub max_coverage: u32,
    pub timeout_secs: u64,
    pub input_size_min: usize,
    pub input_size_max: usize,
    pub corpus_size_limit: usize,
    pub enable_sanitizers: bool,
}

impl Default for FuzzerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_iterations: 10000,
            max_coverage: 10000,
            timeout_secs: 3600,
            input_size_min: 1,
            input_size_max: 4096,
            corpus_size_limit: 10000,
            enable_sanitizers: true,
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
