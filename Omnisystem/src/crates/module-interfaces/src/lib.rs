use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

pub mod error;
pub mod types;

pub use error::ModuleError;
pub use types::*;

#[async_trait]
pub trait ModuleInterface: Send + Sync {
    fn id(&self) -> &str;
    fn version(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn module_type(&self) -> ModuleType;
    fn capabilities(&self) -> Vec<String>;
    fn dependencies(&self) -> Vec<ModuleDependency>;

    async fn initialize(&mut self) -> Result<(), ModuleError>;
    async fn execute(&self, command: &str, args: &str) -> Result<String, ModuleError>;
    async fn shutdown(&mut self) -> Result<(), ModuleError>;

    fn status(&self) -> ModuleStatus;
    fn metadata(&self) -> &ModuleMetadata;
    fn health_check(&self) -> HealthStatus;

    async fn configure(&mut self, config: ModuleConfig) -> Result<(), ModuleError>;
    fn get_config(&self) -> ModuleConfig;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub module_type: ModuleType,
    pub author: Option<String>,
    pub license: Option<String>,
    pub capabilities: Vec<String>,
    pub dependencies: Vec<ModuleDependency>,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModuleType {
    BaseModule,
    FeatureModule,
    AppModule,
    PluginModule,
    UtilityModule,
    DriverModule,
    ProtocolModule,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModuleStatus {
    Unloaded,
    Loading,
    Loaded,
    Running,
    Paused,
    Stopping,
    Error,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleDependency {
    pub module_id: String,
    pub version_range: String,
    pub required: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleConfig {
    pub settings: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub last_check: u64,
    pub uptime_seconds: u64,
    pub error_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleLoadRequest {
    pub module_id: String,
    pub version: Option<String>,
    pub config: Option<ModuleConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleLoadResponse {
    pub success: bool,
    pub module_id: String,
    pub message: String,
    pub load_time_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_type_enum() {
        assert_eq!(ModuleType::BaseModule, ModuleType::BaseModule);
    }

    #[test]
    fn test_module_status_enum() {
        assert_eq!(ModuleStatus::Unloaded, ModuleStatus::Unloaded);
    }

    #[test]
    fn test_module_dependency_creation() {
        let dep = ModuleDependency {
            module_id: "test-module".to_string(),
            version_range: ">=1.0.0".to_string(),
            required: true,
        };
        assert_eq!(dep.module_id, "test-module");
    }

    #[test]
    fn test_module_config_creation() {
        let mut config = ModuleConfig {
            settings: HashMap::new(),
        };
        config.settings.insert("key".to_string(), serde_json::json!("value"));
        assert!(config.settings.contains_key("key"));
    }

    #[test]
    fn test_health_status_creation() {
        let health = HealthStatus {
            healthy: true,
            last_check: 0,
            uptime_seconds: 100,
            error_count: 0,
        };
        assert!(health.healthy);
        assert_eq!(health.uptime_seconds, 100);
    }

    #[test]
    fn test_module_load_request_creation() {
        let request = ModuleLoadRequest {
            module_id: "test-module".to_string(),
            version: Some("1.0.0".to_string()),
            config: None,
        };
        assert_eq!(request.module_id, "test-module");
    }

    #[test]
    fn test_module_load_response_creation() {
        let response = ModuleLoadResponse {
            success: true,
            module_id: "test-module".to_string(),
            message: "Module loaded successfully".to_string(),
            load_time_ms: 50,
        };
        assert!(response.success);
        assert_eq!(response.load_time_ms, 50);
    }

    #[test]
    fn test_module_metadata_creation() {
        let metadata = ModuleMetadata {
            id: "test-module".to_string(),
            name: "Test Module".to_string(),
            version: "1.0.0".to_string(),
            description: "A test module".to_string(),
            module_type: ModuleType::BaseModule,
            author: Some("Author".to_string()),
            license: Some("Apache-2.0".to_string()),
            capabilities: vec!["test".to_string()],
            dependencies: vec![],
            tags: vec!["test".to_string()],
            metadata: HashMap::new(),
        };
        assert_eq!(metadata.id, "test-module");
        assert_eq!(metadata.name, "Test Module");
    }
}
