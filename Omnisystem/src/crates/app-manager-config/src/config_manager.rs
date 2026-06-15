use crate::{AppConfig, Result, ConfigError};
use dashmap::DashMap;
use std::path::Path;
use std::sync::Arc;

pub struct ConfigManager {
    configs: Arc<DashMap<String, AppConfig>>,
}

impl ConfigManager {
    pub fn new() -> Self {
        ConfigManager {
            configs: Arc::new(DashMap::new()),
        }
    }

    pub fn save_config(&self, config: AppConfig) -> Result<()> {
        self.configs.insert(config.app_id.clone(), config.clone());
        tracing::debug!("Saved config for {}", config.app_id);
        Ok(())
    }

    pub fn load_config(&self, app_id: &str) -> Result<AppConfig> {
        self.configs
            .get(app_id)
            .map(|r| r.clone())
            .ok_or_else(|| ConfigError::ConfigNotFound(app_id.to_string()))
    }

    pub fn delete_config(&self, app_id: &str) -> Result<()> {
        self.configs
            .remove(app_id)
            .ok_or_else(|| ConfigError::ConfigNotFound(app_id.to_string()))?;

        Ok(())
    }

    pub fn update_config(&self, app_id: &str, updates: AppConfig) -> Result<()> {
        if self.configs.contains_key(app_id) {
            self.configs.insert(app_id.to_string(), updates);
            tracing::debug!("Updated config for {}", app_id);
            Ok(())
        } else {
            Err(ConfigError::ConfigNotFound(app_id.to_string()))
        }
    }

    pub fn list_all_configs(&self) -> Vec<AppConfig> {
        self.configs.iter().map(|r| r.value().clone()).collect()
    }

    pub async fn load_from_file(&self, path: &Path) -> Result<AppConfig> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ConfigError::IoError(e))?;

        if path.extension().map_or(false, |ext| ext == "json") {
            AppConfig::from_json(&content)
                .map_err(|e| ConfigError::JsonError(e))
        } else if path.extension().map_or(false, |ext| ext == "toml") {
            AppConfig::from_toml(&content)
                .map_err(|e| ConfigError::TomlError(e))
        } else {
            Err(ConfigError::InvalidConfiguration("Unknown file format".to_string()))
        }
    }

    pub async fn save_to_file(&self, app_id: &str, path: &Path) -> Result<()> {
        let config = self.load_config(app_id)?;

        let content = if path.extension().map_or(false, |ext| ext == "json") {
            config.to_json()
                .map_err(|e| ConfigError::JsonError(e))?
        } else if path.extension().map_or(false, |ext| ext == "toml") {
            config.to_toml()
                .map_err(|e| ConfigError::Internal(e.to_string()))?
        } else {
            return Err(ConfigError::InvalidConfiguration("Unknown file format".to_string()));
        };

        tokio::fs::write(path, content)
            .await
            .map_err(|e| ConfigError::IoError(e))
    }

    pub fn get_config_count(&self) -> usize {
        self.configs.len()
    }

    pub fn clear_all(&self) {
        self.configs.clear();
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_load_config() {
        let manager = ConfigManager::new();
        let config = AppConfig::new("test".to_string(), "1.0.0".to_string());

        manager.save_config(config.clone()).unwrap();
        let loaded = manager.load_config("test").unwrap();

        assert_eq!(loaded.app_id, config.app_id);
    }

    #[test]
    fn test_update_config() {
        let manager = ConfigManager::new();
        let config = AppConfig::new("test".to_string(), "1.0.0".to_string());

        manager.save_config(config).unwrap();

        let updated = AppConfig::new("test".to_string(), "2.0.0".to_string());
        manager.update_config("test", updated).unwrap();

        let loaded = manager.load_config("test").unwrap();
        assert_eq!(loaded.version, "2.0.0");
    }

    #[test]
    fn test_delete_config() {
        let manager = ConfigManager::new();
        let config = AppConfig::new("test".to_string(), "1.0.0".to_string());

        manager.save_config(config).unwrap();
        assert!(manager.load_config("test").is_ok());

        manager.delete_config("test").unwrap();
        assert!(manager.load_config("test").is_err());
    }

    #[test]
    fn test_list_all_configs() {
        let manager = ConfigManager::new();

        manager.save_config(AppConfig::new("app1".to_string(), "1.0.0".to_string())).unwrap();
        manager.save_config(AppConfig::new("app2".to_string(), "2.0.0".to_string())).unwrap();

        let configs = manager.list_all_configs();
        assert_eq!(configs.len(), 2);
    }
}
