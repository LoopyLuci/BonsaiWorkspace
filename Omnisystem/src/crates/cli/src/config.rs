/// Configuration management for Omnisystem CLI
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub app_name: String,
    pub version: String,
    pub debug: bool,
    pub log_level: String,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub settings: HashMap<String, String>,
}

impl Config {
    pub fn new(app_name: String) -> Self {
        Self {
            app_name,
            version: env!("CARGO_PKG_VERSION").to_string(),
            debug: false,
            log_level: "info".to_string(),
            data_dir: PathBuf::from(".omnisystem/data"),
            cache_dir: PathBuf::from(".omnisystem/cache"),
            settings: HashMap::new(),
        }
    }

    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    pub fn with_log_level(mut self, level: String) -> Self {
        self.log_level = level;
        self
    }

    pub fn set_setting(&mut self, key: String, value: String) {
        self.settings.insert(key, value);
    }

    pub fn get_setting(&self, key: &str) -> Option<&String> {
        self.settings.get(key)
    }
}

#[derive(Debug, Clone)]
pub struct ConfigProfile {
    pub name: String,
    pub config: Config,
    pub created_at: i64,
    pub last_modified: i64,
}

impl ConfigProfile {
    pub fn new(name: String, config: Config) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            name,
            config,
            created_at: now,
            last_modified: now,
        }
    }
}

pub struct ConfigManager {
    profiles: Arc<RwLock<HashMap<String, ConfigProfile>>>,
    active_profile: Arc<RwLock<String>>,
    config_dir: PathBuf,
}

impl ConfigManager {
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            profiles: Arc::new(RwLock::new(HashMap::new())),
            active_profile: Arc::new(RwLock::new("default".to_string())),
            config_dir,
        }
    }

    pub async fn load_config(&self, profile_name: &str) -> Result<Config> {
        let profiles = self.profiles.read().await;
        if let Some(profile) = profiles.get(profile_name) {
            Ok(profile.config.clone())
        } else {
            Err(anyhow::anyhow!("Profile not found: {}", profile_name))
        }
    }

    pub async fn save_config(&self, profile_name: String, config: Config) -> Result<()> {
        let mut profiles = self.profiles.write().await;
        let profile = ConfigProfile::new(profile_name.clone(), config);
        profiles.insert(profile_name, profile);

        tracing::info!("Saved configuration profile");
        Ok(())
    }

    pub async fn create_profile(&self, name: String, config: Config) -> Result<String> {
        let mut profiles = self.profiles.write().await;
        let profile = ConfigProfile::new(name.clone(), config);
        profiles.insert(name.clone(), profile);

        tracing::info!("Created configuration profile: {}", name);
        Ok(name)
    }

    pub async fn delete_profile(&self, name: &str) -> Result<()> {
        let mut profiles = self.profiles.write().await;
        profiles.remove(name);

        tracing::info!("Deleted configuration profile: {}", name);
        Ok(())
    }

    pub async fn list_profiles(&self) -> Result<Vec<String>> {
        let profiles = self.profiles.read().await;
        let names: Vec<String> = profiles.keys().cloned().collect();
        Ok(names)
    }

    pub async fn set_active_profile(&self, name: String) -> Result<()> {
        let profiles = self.profiles.read().await;
        if !profiles.contains_key(&name) {
            return Err(anyhow::anyhow!("Profile not found: {}", name));
        }

        let mut active = self.active_profile.write().await;
        *active = name;

        tracing::info!("Set active profile");
        Ok(())
    }

    pub async fn get_active_profile(&self) -> Result<String> {
        let active = self.active_profile.read().await;
        Ok(active.clone())
    }

    pub async fn get_active_config(&self) -> Result<Config> {
        let active_name = self.get_active_profile().await?;
        self.load_config(&active_name).await
    }

    pub async fn update_setting(&self, profile_name: &str, key: String, value: String) -> Result<()> {
        let mut profiles = self.profiles.write().await;
        if let Some(profile) = profiles.get_mut(profile_name) {
            profile.config.set_setting(key, value);
            profile.last_modified = chrono::Utc::now().timestamp();
        }

        Ok(())
    }

    pub async fn get_setting(&self, profile_name: &str, key: &str) -> Result<Option<String>> {
        let profiles = self.profiles.read().await;
        if let Some(profile) = profiles.get(profile_name) {
            Ok(profile.config.get_setting(key).cloned())
        } else {
            Err(anyhow::anyhow!("Profile not found: {}", profile_name))
        }
    }

    pub async fn export_config(&self, profile_name: &str) -> Result<String> {
        let profiles = self.profiles.read().await;
        if let Some(profile) = profiles.get(profile_name) {
            let json = serde_json::to_string_pretty(&profile.config)?;
            Ok(json)
        } else {
            Err(anyhow::anyhow!("Profile not found: {}", profile_name))
        }
    }

    pub async fn import_config(&self, profile_name: String, json: &str) -> Result<()> {
        let config: Config = serde_json::from_str(json)?;
        self.save_config(profile_name, config).await
    }

    pub async fn reset_profile(&self, name: &str) -> Result<()> {
        let mut profiles = self.profiles.write().await;
        if let Some(profile) = profiles.get_mut(name) {
            profile.config = Config::new(profile.config.app_name.clone());
            profile.last_modified = chrono::Utc::now().timestamp();
        }

        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new(PathBuf::from(".omnisystem/config"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config_creation() {
        let config = Config::new("test".to_string());
        assert_eq!(config.app_name, "test");
        assert!(!config.debug);
    }

    #[tokio::test]
    async fn test_config_manager() {
        let manager = ConfigManager::default();
        let config = Config::new("app".to_string());

        let result = manager.create_profile("test".to_string(), config).await;
        assert!(result.is_ok());

        let profiles = manager.list_profiles().await.unwrap();
        assert!(profiles.contains(&"test".to_string()));
    }

    #[tokio::test]
    async fn test_set_active_profile() {
        let manager = ConfigManager::default();
        let config = Config::new("app".to_string());

        manager.create_profile("profile1".to_string(), config).await.unwrap();
        let result = manager.set_active_profile("profile1".to_string()).await;

        assert!(result.is_ok());
        let active = manager.get_active_profile().await.unwrap();
        assert_eq!(active, "profile1");
    }

    #[test]
    fn test_config_settings() {
        let mut config = Config::new("test".to_string());
        config.set_setting("key1".to_string(), "value1".to_string());

        let value = config.get_setting("key1");
        assert_eq!(value, Some(&"value1".to_string()));
    }
}
