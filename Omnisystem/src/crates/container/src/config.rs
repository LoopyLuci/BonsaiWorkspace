use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BcfConfig {
    pub node_id: String,
    pub namespace: String,
    pub cache_dir: PathBuf,
    pub registry_url: String,
    pub enable_universe_logging: bool,
    pub enable_survival_system: bool,
    pub scheduling_interval_ms: u64,
    pub health_check_interval_secs: u64,
    pub log_level: String,
    pub max_containers_per_node: usize,
    pub default_memory_mib: u64,
    pub default_cpu_cores: f64,
}

impl Default for BcfConfig {
    fn default() -> Self {
        Self {
            node_id: format!("node-{}", uuid::Uuid::new_v4()),
            namespace: "default".to_string(),
            cache_dir: "/var/lib/container".into(),
            registry_url: "echo://registry.bonsai".to_string(),
            enable_universe_logging: true,
            enable_survival_system: true,
            scheduling_interval_ms: 100,
            health_check_interval_secs: 10,
            log_level: "info".to_string(),
            max_containers_per_node: 256,
            default_memory_mib: 512,
            default_cpu_cores: 1.0,
        }
    }
}
