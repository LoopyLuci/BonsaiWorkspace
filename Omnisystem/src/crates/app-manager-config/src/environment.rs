use crate::{Result, ConfigError};
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;

pub struct EnvironmentManager {
    environments: Arc<DashMap<String, HashMap<String, String>>>,
}

impl EnvironmentManager {
    pub fn new() -> Self {
        EnvironmentManager {
            environments: Arc::new(DashMap::new()),
        }
    }

    pub fn create_environment(&self, app_id: &str, env: HashMap<String, String>) -> Result<()> {
        self.environments.insert(app_id.to_string(), env);
        tracing::debug!("Created environment for {}", app_id);
        Ok(())
    }

    pub fn get_environment(&self, app_id: &str) -> Result<HashMap<String, String>> {
        self.environments
            .get(app_id)
            .map(|r| r.clone())
            .ok_or_else(|| ConfigError::ConfigNotFound(app_id.to_string()))
    }

    pub fn set_variable(&self, app_id: &str, key: String, value: String) -> Result<()> {
        if let Some(mut env) = self.environments.get_mut(app_id) {
            env.insert(key, value);
            Ok(())
        } else {
            let mut env = HashMap::new();
            env.insert(key, value);
            self.environments.insert(app_id.to_string(), env);
            Ok(())
        }
    }

    pub fn get_variable(&self, app_id: &str, key: &str) -> Result<Option<String>> {
        self.environments
            .get(app_id)
            .map(|env| env.get(key).cloned())
            .ok_or_else(|| ConfigError::ConfigNotFound(app_id.to_string()))
    }

    pub fn remove_variable(&self, app_id: &str, key: &str) -> Result<()> {
        if let Some(mut env) = self.environments.get_mut(app_id) {
            env.remove(key);
            Ok(())
        } else {
            Err(ConfigError::ConfigNotFound(app_id.to_string()))
        }
    }

    pub fn clear_environment(&self, app_id: &str) -> Result<()> {
        self.environments
            .remove(app_id)
            .ok_or_else(|| ConfigError::ConfigNotFound(app_id.to_string()))?;

        Ok(())
    }

    pub fn merge_environment(&self, app_id: &str, additional: HashMap<String, String>) -> Result<()> {
        if let Some(mut env) = self.environments.get_mut(app_id) {
            for (key, value) in additional {
                env.insert(key, value);
            }
            Ok(())
        } else {
            self.environments.insert(app_id.to_string(), additional);
            Ok(())
        }
    }

    pub fn list_all_environments(&self) -> Vec<(String, HashMap<String, String>)> {
        self.environments
            .iter()
            .map(|r| (r.key().clone(), r.value().clone()))
            .collect()
    }

    pub fn export_to_shell(&self, app_id: &str) -> Result<String> {
        let env = self.get_environment(app_id)?;
        let lines: Vec<String> = env
            .iter()
            .map(|(k, v)| format!("export {}=\"{}\"", k, v.replace("\"", "\\\"")))
            .collect();

        Ok(lines.join("\n"))
    }
}

impl Default for EnvironmentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_environment() {
        let manager = EnvironmentManager::new();
        let mut env = HashMap::new();
        env.insert("KEY".to_string(), "value".to_string());

        manager.create_environment("app", env).unwrap();
        let loaded = manager.get_environment("app").unwrap();

        assert_eq!(loaded.get("KEY"), Some(&"value".to_string()));
    }

    #[test]
    fn test_set_variable() {
        let manager = EnvironmentManager::new();
        manager.set_variable("app", "KEY".to_string(), "value".to_string()).unwrap();

        let value = manager.get_variable("app", "KEY").unwrap();
        assert_eq!(value, Some("value".to_string()));
    }

    #[test]
    fn test_remove_variable() {
        let manager = EnvironmentManager::new();
        manager.set_variable("app", "KEY".to_string(), "value".to_string()).unwrap();
        manager.remove_variable("app", "KEY").unwrap();

        let value = manager.get_variable("app", "KEY").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_merge_environment() {
        let manager = EnvironmentManager::new();

        let mut env1 = HashMap::new();
        env1.insert("KEY1".to_string(), "value1".to_string());
        manager.create_environment("app", env1).unwrap();

        let mut env2 = HashMap::new();
        env2.insert("KEY2".to_string(), "value2".to_string());
        manager.merge_environment("app", env2).unwrap();

        let env = manager.get_environment("app").unwrap();
        assert_eq!(env.len(), 2);
    }

    #[test]
    fn test_export_to_shell() {
        let manager = EnvironmentManager::new();
        manager.set_variable("app", "KEY".to_string(), "value".to_string()).unwrap();

        let shell = manager.export_to_shell("app").unwrap();
        assert!(shell.contains("export KEY=\"value\""));
    }
}
