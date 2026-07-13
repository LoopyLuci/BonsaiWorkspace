use crate::{ConfigError, ConfigResult, ConfigValue};
use dashmap::DashMap;
use std::sync::Arc;

pub struct ConfigManager {
    configs: Arc<DashMap<String, ConfigValue>>,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self {
            configs: Arc::new(DashMap::new()),
        }
    }

    pub async fn set_config(&self, config: &ConfigValue) -> ConfigResult<()> {
        self.configs.insert(config.key.clone(), config.clone());
        Ok(())
    }

    pub async fn get_config(&self, key: &str) -> ConfigResult<ConfigValue> {
        self.configs
            .get(key)
            .map(|entry| entry.clone())
            .ok_or(ConfigError::ConfigNotFound)
    }

    pub fn config_count(&self) -> usize {
        self.configs.len()
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

    #[tokio::test]
    async fn test_set_config() {
        let mgr = ConfigManager::new();
        let cfg = ConfigValue {
            key: "db_host".to_string(),
            value: "localhost".to_string(),
        };

        mgr.set_config(&cfg).await.unwrap();
        assert_eq!(mgr.config_count(), 1);
    }

    #[tokio::test]
    async fn test_get_config() {
        let mgr = ConfigManager::new();
        let cfg = ConfigValue {
            key: "db_host".to_string(),
            value: "localhost".to_string(),
        };

        mgr.set_config(&cfg).await.unwrap();
        let retrieved = mgr.get_config("db_host").await.unwrap();
        assert_eq!(retrieved.value, "localhost");
    }
}
