//! Configuration for bonsai-bedf-property

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PropertyTestConfig {
    pub enabled: bool,
    pub num_tests: usize,
    pub max_shrink_iterations: usize,
    pub timeout_secs: u64,
}

impl Default for PropertyTestConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            num_tests: 100,
            max_shrink_iterations: 100,
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
