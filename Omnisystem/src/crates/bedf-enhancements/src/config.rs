//! Configuration for bonsai-bedf-enhancements

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnhancementsConfig {
    pub enabled: bool,
    pub enable_resource_budgeting: bool,
    pub enable_flaky_detection: bool,
    pub enable_supply_chain: bool,
    pub enable_quantum_resistant: bool,
    pub enable_cross_language: bool,
    pub enable_llm_fixes: bool,
    pub enable_etl: bool,
    pub enable_stateful_pentest: bool,
    pub enable_hardened_sandbox: bool,
    pub enable_knowledge_distillation: bool,
}

impl Default for EnhancementsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enable_resource_budgeting: true,
            enable_flaky_detection: true,
            enable_supply_chain: true,
            enable_quantum_resistant: true,
            enable_cross_language: true,
            enable_llm_fixes: true,
            enable_etl: true,
            enable_stateful_pentest: true,
            enable_hardened_sandbox: true,
            enable_knowledge_distillation: true,
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
