use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogOutput {
    Console,
    File(PathBuf),
    Both,
    RemoteServer(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProcessPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    pub cpu_cores: u32,
    pub memory_mb: u32,
    pub disk_quota_mb: u32,
    pub priority: ProcessPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app_id: String,
    pub version: String,

    pub environment: HashMap<String, String>,
    pub features: HashMap<String, bool>,

    pub resources: ResourceAllocation,

    pub startup_order: u32,
    pub auto_restart: bool,
    pub restart_delay_secs: u32,
    pub max_restarts: u32,

    pub log_level: LogLevel,
    pub log_output: LogOutput,

    pub health_check_interval_secs: u32,
    pub error_reporting_enabled: bool,

    pub data_directory: PathBuf,
    pub cache_directory: PathBuf,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AppConfig {
    pub fn new(app_id: String, version: String) -> Self {
        let now = Utc::now();
        AppConfig {
            app_id,
            version,
            environment: HashMap::new(),
            features: HashMap::new(),
            resources: ResourceAllocation {
                cpu_cores: 2,
                memory_mb: 512,
                disk_quota_mb: 1024,
                priority: ProcessPriority::Normal,
            },
            startup_order: 0,
            auto_restart: false,
            restart_delay_secs: 5,
            max_restarts: 3,
            log_level: LogLevel::Info,
            log_output: LogOutput::Console,
            health_check_interval_secs: 30,
            error_reporting_enabled: true,
            data_directory: PathBuf::from("/data"),
            cache_directory: PathBuf::from("/cache"),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn set_env(&mut self, key: String, value: String) -> &mut Self {
        self.environment.insert(key, value);
        self.updated_at = Utc::now();
        self
    }

    pub fn set_feature(&mut self, name: String, enabled: bool) -> &mut Self {
        self.features.insert(name, enabled);
        self.updated_at = Utc::now();
        self
    }

    pub fn set_resources(&mut self, cpu_cores: u32, memory_mb: u32, disk_quota_mb: u32) -> &mut Self {
        self.resources.cpu_cores = cpu_cores;
        self.resources.memory_mb = memory_mb;
        self.resources.disk_quota_mb = disk_quota_mb;
        self.updated_at = Utc::now();
        self
    }

    pub fn get_env(&self, key: &str) -> Option<String> {
        self.environment.get(key).cloned()
    }

    pub fn is_feature_enabled(&self, name: &str) -> bool {
        self.features.get(name).copied().unwrap_or(false)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string(self)
    }

    pub fn from_toml(toml: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_creation() {
        let config = AppConfig::new("test-app".to_string(), "1.0.0".to_string());
        assert_eq!(config.app_id, "test-app");
        assert_eq!(config.version, "1.0.0");
    }

    #[test]
    fn test_set_env() {
        let mut config = AppConfig::new("test".to_string(), "1.0.0".to_string());
        config.set_env("KEY".to_string(), "value".to_string());

        assert_eq!(config.get_env("KEY"), Some("value".to_string()));
    }

    #[test]
    fn test_set_feature() {
        let mut config = AppConfig::new("test".to_string(), "1.0.0".to_string());
        config.set_feature("feature1".to_string(), true);

        assert!(config.is_feature_enabled("feature1"));
        assert!(!config.is_feature_enabled("feature2"));
    }

    #[test]
    fn test_serialization() {
        let config = AppConfig::new("test".to_string(), "1.0.0".to_string());
        let json = config.to_json().unwrap();
        let deserialized = AppConfig::from_json(&json).unwrap();

        assert_eq!(deserialized.app_id, config.app_id);
    }
}
