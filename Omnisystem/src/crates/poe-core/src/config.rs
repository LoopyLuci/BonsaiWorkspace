use serde::{Serialize, Deserialize};
use crate::personality::PersonalityConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoeConfig {
    pub identity_path: String,
    pub model_path: String,
    pub journal_path: String,
    pub personality: PersonalityConfig,
    pub council_nodes: Vec<String>,
    pub governance_threshold: usize,
    pub mesh_enabled: bool,
    pub capability_token: String,
}

impl Default for PoeConfig {
    fn default() -> Self {
        Self {
            identity_path: "/var/lib/poe/identity.hash".into(),
            model_path: "bonsai://models/poe-empathy-v1.bkp".into(),
            journal_path: "/var/lib/poe/journal.kdb".into(),
            personality: PersonalityConfig::default(),
            council_nodes: vec!["node-1".into(), "node-2".into(), "node-3".into()],
            governance_threshold: 2,
            mesh_enabled: true,
            capability_token: "PoeCap:inference".into(),
        }
    }
}
