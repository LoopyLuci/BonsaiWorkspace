use crate::{Result, SecurityError};
use crate::permission_manager::SandboxLevel;
use app_manager_core::AppId;
use dashmap::DashMap;
use std::sync::Arc;

pub struct SandboxManager {
    sandboxes: Arc<DashMap<AppId, SandboxConfig>>,
}

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub app_id: AppId,
    pub level: SandboxLevel,
    pub allowed_paths: Vec<String>,
    pub allowed_hosts: Vec<String>,
    pub resource_limit_memory_mb: u32,
    pub resource_limit_cpu_percent: u32,
}

impl SandboxManager {
    pub fn new() -> Self {
        SandboxManager {
            sandboxes: Arc::new(DashMap::new()),
        }
    }

    pub fn create_sandbox(&self, config: SandboxConfig) -> Result<()> {
        tracing::info!("Creating sandbox for {}", config.app_id);
        self.sandboxes.insert(config.app_id.clone(), config);
        Ok(())
    }

    pub fn get_sandbox(&self, app_id: &AppId) -> Result<SandboxConfig> {
        self.sandboxes
            .get(app_id)
            .map(|r| r.clone())
            .ok_or_else(|| SecurityError::SandboxViolation("Sandbox not found".to_string()))
    }

    pub fn check_path_access(&self, app_id: &AppId, path: &str) -> Result<bool> {
        let config = self.get_sandbox(app_id)?;

        match config.level {
            SandboxLevel::Unrestricted => Ok(true),
            SandboxLevel::Basic => {
                Ok(config.allowed_paths.iter().any(|p| path.starts_with(p)))
            }
            SandboxLevel::Strict => {
                Ok(config.allowed_paths.iter().any(|p| path.starts_with(p)))
            }
            SandboxLevel::Isolated => {
                Ok(config.allowed_paths.iter().any(|p| path.starts_with(p)))
            }
        }
    }

    pub fn check_host_access(&self, app_id: &AppId, host: &str) -> Result<bool> {
        let config = self.get_sandbox(app_id)?;

        match config.level {
            SandboxLevel::Unrestricted => Ok(true),
            SandboxLevel::Basic => {
                Ok(config.allowed_hosts.iter().any(|h| host.contains(h)))
            }
            SandboxLevel::Strict => {
                Ok(config.allowed_hosts.iter().any(|h| host == h))
            }
            SandboxLevel::Isolated => {
                Ok(config.allowed_hosts.is_empty() || config.allowed_hosts.iter().any(|h| host == h))
            }
        }
    }

    pub fn update_sandbox(&self, config: SandboxConfig) -> Result<()> {
        tracing::info!("Updating sandbox for {}", config.app_id);
        self.sandboxes.insert(config.app_id.clone(), config);
        Ok(())
    }

    pub fn remove_sandbox(&self, app_id: &AppId) -> Result<()> {
        self.sandboxes
            .remove(app_id)
            .ok_or_else(|| SecurityError::SandboxViolation("Sandbox not found".to_string()))?;

        Ok(())
    }

    pub fn list_all_sandboxes(&self) -> Vec<SandboxConfig> {
        self.sandboxes.iter().map(|r| r.value().clone()).collect()
    }
}

impl Default for SandboxManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sandbox() {
        let manager = SandboxManager::new();
        let config = SandboxConfig {
            app_id: AppId::new("test").unwrap(),
            level: SandboxLevel::Basic,
            allowed_paths: vec!["/app".to_string()],
            allowed_hosts: vec!["example.com".to_string()],
            resource_limit_memory_mb: 512,
            resource_limit_cpu_percent: 50,
        };

        manager.create_sandbox(config).unwrap();
        assert_eq!(manager.list_all_sandboxes().len(), 1);
    }

    #[test]
    fn test_check_path_access() {
        let manager = SandboxManager::new();
        let app_id = AppId::new("test").unwrap();
        let config = SandboxConfig {
            app_id: app_id.clone(),
            level: SandboxLevel::Basic,
            allowed_paths: vec!["/app".to_string()],
            allowed_hosts: Vec::new(),
            resource_limit_memory_mb: 512,
            resource_limit_cpu_percent: 50,
        };

        manager.create_sandbox(config).unwrap();
        assert!(manager.check_path_access(&app_id, "/app/data").unwrap());
        assert!(!manager.check_path_access(&app_id, "/etc/passwd").unwrap());
    }

    #[test]
    fn test_check_host_access() {
        let manager = SandboxManager::new();
        let app_id = AppId::new("test").unwrap();
        let config = SandboxConfig {
            app_id: app_id.clone(),
            level: SandboxLevel::Basic,
            allowed_paths: Vec::new(),
            allowed_hosts: vec!["example.com".to_string()],
            resource_limit_memory_mb: 512,
            resource_limit_cpu_percent: 50,
        };

        manager.create_sandbox(config).unwrap();
        assert!(manager.check_host_access(&app_id, "example.com").unwrap());
    }
}
