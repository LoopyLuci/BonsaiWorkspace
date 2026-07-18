use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enhancement {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

pub struct EnhancementEngine {
    enhancements: Vec<Enhancement>,
}

impl EnhancementEngine {
    pub fn new(config: super::EnhancementsConfig) -> Self {
        let enhancements = vec![
            Enhancement {
                id: 1,
                name: "Resource-Aware Fuzzing".to_string(),
                description: "Budget CPU, memory, disk, and time".to_string(),
                enabled: config.enable_resource_budgeting,
            },
            Enhancement {
                id: 2,
                name: "Flaky Test Detection".to_string(),
                description: "Quarantine and analyze flaky tests".to_string(),
                enabled: config.enable_flaky_detection,
            },
            Enhancement {
                id: 3,
                name: "Supply Chain Attack Detection".to_string(),
                description: "Detect malicious dependencies".to_string(),
                enabled: config.enable_supply_chain,
            },
            Enhancement {
                id: 4,
                name: "Quantum-Resistant Fuzzing".to_string(),
                description: "Test PQC implementations".to_string(),
                enabled: config.enable_quantum_resistant,
            },
            Enhancement {
                id: 5,
                name: "Cross-Language Fuzzing".to_string(),
                description: "Fuzz Rust/C/Python code".to_string(),
                enabled: config.enable_cross_language,
            },
            Enhancement {
                id: 6,
                name: "LLM Fix Variants".to_string(),
                description: "Generate multiple fix suggestions".to_string(),
                enabled: config.enable_llm_fixes,
            },
            Enhancement {
                id: 7,
                name: "ETL Optimization".to_string(),
                description: "Self-tuning fuzzer parameters".to_string(),
                enabled: config.enable_etl,
            },
            Enhancement {
                id: 8,
                name: "Stateful Pen-testing".to_string(),
                description: "Sequence-aware API testing".to_string(),
                enabled: config.enable_stateful_pentest,
            },
            Enhancement {
                id: 9,
                name: "Hardened Sandboxes".to_string(),
                description: "Seccomp + Landlock isolation".to_string(),
                enabled: config.enable_hardened_sandbox,
            },
            Enhancement {
                id: 10,
                name: "Knowledge Distillation".to_string(),
                description: "Cross-project pattern sharing".to_string(),
                enabled: config.enable_knowledge_distillation,
            },
        ];

        Self { enhancements }
    }

    pub fn list_enhancements(&self) -> Vec<Enhancement> {
        self.enhancements.clone()
    }

    pub fn get_enabled(&self) -> Vec<Enhancement> {
        self.enhancements
            .iter()
            .filter(|e| e.enabled)
            .cloned()
            .collect()
    }

    pub fn get_by_id(&self, id: u32) -> Option<Enhancement> {
        self.enhancements.iter().find(|e| e.id == id).cloned()
    }

    pub fn is_enabled(&self, name: &str) -> bool {
        self.enhancements.iter().any(|e| e.name == name && e.enabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhancements_list() {
        let config = super::super::EnhancementsConfig::default();
        let engine = EnhancementEngine::new(config);
        assert_eq!(engine.list_enhancements().len(), 10);
    }

    #[test]
    fn test_default_config_enables_everything() {
        let config = super::super::EnhancementsConfig::default();
        let engine = EnhancementEngine::new(config);
        assert_eq!(engine.get_enabled().len(), 10);
    }

    #[test]
    fn test_disabled_flag_excludes_specific_enhancement() {
        let mut config = super::super::EnhancementsConfig::default();
        config.enable_quantum_resistant = false;
        config.enable_llm_fixes = false;

        let engine = EnhancementEngine::new(config);
        let enabled = engine.get_enabled();

        assert_eq!(enabled.len(), 8);
        assert!(!engine.is_enabled("Quantum-Resistant Fuzzing"));
        assert!(!engine.is_enabled("LLM Fix Variants"));
        assert!(engine.is_enabled("Resource-Aware Fuzzing"));
    }

    #[test]
    fn test_get_by_id() {
        let config = super::super::EnhancementsConfig::default();
        let engine = EnhancementEngine::new(config);

        let found = engine.get_by_id(3).unwrap();
        assert_eq!(found.name, "Supply Chain Attack Detection");
        assert!(engine.get_by_id(999).is_none());
    }
}
