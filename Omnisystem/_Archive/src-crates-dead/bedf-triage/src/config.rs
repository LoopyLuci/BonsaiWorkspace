//! Configuration for bonsai-bedf-triage

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriageConfig {
    pub enabled: bool,
    pub enable_ai_fixes: bool,
    pub max_fix_suggestions: usize,
}

impl Default for TriageConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enable_ai_fixes: true,
            max_fix_suggestions: 5,
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
