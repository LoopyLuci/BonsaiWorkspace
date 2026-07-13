//! Configuration management for the Anti-Hallucination Gateway

use crate::error::{GatewayError, GatewayResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod policy;
pub use policy::PolicyConfig;

/// Version information for configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigVersion {
    /// Version number
    pub version: u32,
    /// Timestamp when this version was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Description of changes
    pub description: String,
}

impl ConfigVersion {
    pub fn new(version: u32, description: String) -> Self {
        Self {
            version,
            created_at: chrono::Utc::now(),
            description,
        }
    }
}

/// Main configuration for AHF Gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AhfConfig {
    /// Knowledge grounding configuration
    pub knowledge_sources: Vec<String>,

    /// Verification thresholds
    pub grounding_threshold: f64,
    pub confidence_threshold: f64,
    pub bias_threshold: f64,

    /// Timeouts (in milliseconds)
    pub pipeline_timeout_ms: u64,
    pub kgs_timeout_ms: u64,
    pub verification_timeout_ms: u64,

    /// Policy configuration
    pub policy: PolicyConfig,

    /// Bias detection patterns
    pub bias_patterns: Vec<String>,

    /// Enable shadow mode for ML bias detection
    pub bias_detector_shadow_mode: bool,

    /// Configuration version
    pub version: ConfigVersion,

    /// Metadata
    pub metadata: HashMap<String, String>,
}

impl Default for AhfConfig {
    fn default() -> Self {
        Self {
            knowledge_sources: vec!["cas".to_string(), "ums".to_string()],
            grounding_threshold: 0.7,
            confidence_threshold: 0.6,
            bias_threshold: 0.5,
            pipeline_timeout_ms: 50,
            kgs_timeout_ms: 20,
            verification_timeout_ms: 20,
            policy: PolicyConfig::default(),
            bias_patterns: vec![],
            bias_detector_shadow_mode: false,
            version: ConfigVersion::new(1, "Initial configuration".to_string()),
            metadata: HashMap::new(),
        }
    }
}

impl AhfConfig {
    /// Create a new configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from a JSON file
    pub async fn load_from_file(path: &Path) -> GatewayResult<Self> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| GatewayError::config_error(format!("Failed to read config file: {}", e)))?;

        serde_json::from_str(&content)
            .map_err(|e| GatewayError::config_error(format!("Invalid config JSON: {}", e)))
    }

    /// Save configuration to a JSON file
    pub async fn save_to_file(&self, path: &Path) -> GatewayResult<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| GatewayError::config_error(format!("Failed to serialize config: {}", e)))?;

        tokio::fs::write(path, content)
            .await
            .map_err(|e| GatewayError::config_error(format!("Failed to write config file: {}", e)))
    }

    /// Validate configuration
    pub fn validate(&self) -> GatewayResult<()> {
        if self.grounding_threshold < 0.0 || self.grounding_threshold > 1.0 {
            return Err(GatewayError::config_error(
                "grounding_threshold must be between 0.0 and 1.0",
            ));
        }

        if self.confidence_threshold < 0.0 || self.confidence_threshold > 1.0 {
            return Err(GatewayError::config_error(
                "confidence_threshold must be between 0.0 and 1.0",
            ));
        }

        if self.bias_threshold < 0.0 || self.bias_threshold > 1.0 {
            return Err(GatewayError::config_error(
                "bias_threshold must be between 0.0 and 1.0",
            ));
        }

        if self.pipeline_timeout_ms == 0 {
            return Err(GatewayError::config_error(
                "pipeline_timeout_ms must be greater than 0",
            ));
        }

        self.policy.validate()?;

        Ok(())
    }

    /// Get a metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|v| v.as_str())
    }

    /// Set a metadata value
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Update version with description
    pub fn update_version(&mut self, description: String) {
        self.version.version += 1;
        self.version = ConfigVersion::new(self.version.version, description);
    }
}

/// Configuration manager with version tracking
pub struct ConfigManager {
    current: Arc<RwLock<AhfConfig>>,
    history: Arc<RwLock<Vec<AhfConfig>>>,
}

impl ConfigManager {
    pub fn new(config: AhfConfig) -> Self {
        Self {
            current: Arc::new(RwLock::new(config)),
            history: Arc::new(RwLock::new(vec![])),
        }
    }

    /// Get current configuration
    pub async fn get(&self) -> AhfConfig {
        self.current.read().await.clone()
    }

    /// Update configuration
    pub async fn update(&self, config: AhfConfig) -> GatewayResult<()> {
        config.validate()?;

        let mut current = self.current.write().await;
        let mut history = self.history.write().await;

        history.push(current.clone());
        *current = config;

        Ok(())
    }

    /// Get configuration history
    pub async fn get_history(&self) -> Vec<AhfConfig> {
        self.history.read().await.clone()
    }

    /// Rollback to previous version
    pub async fn rollback(&self) -> GatewayResult<()> {
        let mut history = self.history.write().await;

        if let Some(previous) = history.pop() {
            let mut current = self.current.write().await;
            *current = previous;
            Ok(())
        } else {
            Err(GatewayError::config_error("No previous configuration to rollback to"))
        }
    }

    /// Get specific version from history
    pub async fn get_version(&self, index: usize) -> GatewayResult<AhfConfig> {
        let history = self.history.read().await;
        history.get(index)
            .cloned()
            .ok_or_else(|| GatewayError::config_error(format!("Version {} not found", index)))
    }
}

impl Clone for ConfigManager {
    fn clone(&self) -> Self {
        Self {
            current: Arc::clone(&self.current),
            history: Arc::clone(&self.history),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let cfg = AhfConfig::default();
        assert_eq!(cfg.grounding_threshold, 0.7);
        assert_eq!(cfg.pipeline_timeout_ms, 50);
    }

    #[test]
    fn test_config_validation() {
        let mut cfg = AhfConfig::default();
        assert!(cfg.validate().is_ok());

        cfg.grounding_threshold = 1.5;
        assert!(cfg.validate().is_err());

        cfg.grounding_threshold = 0.7;
        cfg.pipeline_timeout_ms = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_config_metadata() {
        let mut cfg = AhfConfig::default();
        cfg.set_metadata("key1".to_string(), "value1".to_string());
        assert_eq!(cfg.get_metadata("key1"), Some("value1"));
        assert_eq!(cfg.get_metadata("nonexistent"), None);
    }

    #[test]
    fn test_config_version_update() {
        let mut cfg = AhfConfig::default();
        assert_eq!(cfg.version.version, 1);
        cfg.update_version("Updated config".to_string());
        assert_eq!(cfg.version.version, 2);
        assert_eq!(cfg.version.description, "Updated config");
    }

    #[tokio::test]
    async fn test_config_manager_update() {
        let cfg = AhfConfig::default();
        let manager = ConfigManager::new(cfg);

        let mut new_cfg = manager.get().await;
        new_cfg.grounding_threshold = 0.8;

        assert!(manager.update(new_cfg).await.is_ok());
        let updated = manager.get().await;
        assert_eq!(updated.grounding_threshold, 0.8);
    }

    #[tokio::test]
    async fn test_config_manager_rollback() {
        let cfg = AhfConfig::default();
        let manager = ConfigManager::new(cfg.clone());

        let mut new_cfg = manager.get().await;
        new_cfg.grounding_threshold = 0.9;
        assert!(manager.update(new_cfg).await.is_ok());

        assert!(manager.rollback().await.is_ok());
        let rolled_back = manager.get().await;
        assert_eq!(rolled_back.grounding_threshold, 0.7);
    }

    #[tokio::test]
    async fn test_config_manager_history() {
        let cfg = AhfConfig::default();
        let manager = ConfigManager::new(cfg);

        let mut cfg1 = manager.get().await;
        cfg1.grounding_threshold = 0.75;
        manager.update(cfg1).await.unwrap();

        let mut cfg2 = manager.get().await;
        cfg2.grounding_threshold = 0.8;
        manager.update(cfg2).await.unwrap();

        let history = manager.get_history().await;
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_config_serialization() {
        let cfg = AhfConfig::default();
        let json = serde_json::to_string(&cfg).expect("serialization failed");
        let cfg2: AhfConfig = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(cfg.grounding_threshold, cfg2.grounding_threshold);
    }
}
