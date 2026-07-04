use crate::{Result, SecurityError};
use app_manager_core::AppId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use dashmap::DashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Permission {
    FilesystemRead,
    FilesystemWrite,
    FilesystemDelete,
    NetworkAccess,
    ProcessExecution,
    SystemInfo,
    GpuAccess,
    DatabaseAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub app_id: AppId,
    pub permissions: HashSet<Permission>,
    pub resource_limits: ResourceLimits,
    pub sandbox_level: SandboxLevel,
    pub trusted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub cpu_percent: u32,
    pub memory_mb: u32,
    pub disk_quota_mb: u32,
    pub network_bandwidth_mbps: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SandboxLevel {
    Unrestricted,
    Basic,
    Strict,
    Isolated,
}

pub struct PermissionManager {
    policies: DashMap<AppId, SecurityPolicy>,
}

impl PermissionManager {
    pub fn new() -> Self {
        PermissionManager {
            policies: DashMap::new(),
        }
    }

    pub fn register_policy(&self, policy: SecurityPolicy) -> Result<()> {
        self.policies.insert(policy.app_id.clone(), policy);
        tracing::debug!("Registered security policy");
        Ok(())
    }

    pub fn get_policy(&self, app_id: &AppId) -> Result<SecurityPolicy> {
        self.policies
            .get(app_id)
            .map(|r| r.clone())
            .ok_or_else(|| SecurityError::AccessDenied(app_id.to_string()))
    }

    pub fn has_permission(&self, app_id: &AppId, permission: Permission) -> Result<bool> {
        let policy = self.get_policy(app_id)?;
        Ok(policy.permissions.contains(&permission))
    }

    pub fn grant_permission(&self, app_id: &AppId, permission: Permission) -> Result<()> {
        if let Some(mut policy) = self.policies.get_mut(app_id) {
            policy.permissions.insert(permission);
            tracing::info!("Granted permission to {}", app_id);
            Ok(())
        } else {
            Err(SecurityError::AccessDenied(app_id.to_string()))
        }
    }

    pub fn revoke_permission(&self, app_id: &AppId, permission: Permission) -> Result<()> {
        if let Some(mut policy) = self.policies.get_mut(app_id) {
            policy.permissions.remove(&permission);
            tracing::info!("Revoked permission from {}", app_id);
            Ok(())
        } else {
            Err(SecurityError::AccessDenied(app_id.to_string()))
        }
    }

    pub fn set_sandbox_level(&self, app_id: &AppId, level: SandboxLevel) -> Result<()> {
        if let Some(mut policy) = self.policies.get_mut(app_id) {
            policy.sandbox_level = level;
            tracing::info!("Set sandbox level for {}: {:?}", app_id, level);
            Ok(())
        } else {
            Err(SecurityError::AccessDenied(app_id.to_string()))
        }
    }

    pub fn validate_access(&self, app_id: &AppId, permission: Permission) -> Result<()> {
        let policy = self.get_policy(app_id)?;

        if !policy.permissions.contains(&permission) {
            return Err(SecurityError::PermissionDenied(format!(
                "{:?}",
                permission
            )));
        }

        Ok(())
    }

    pub fn list_all_policies(&self) -> Vec<SecurityPolicy> {
        self.policies.iter().map(|r| r.value().clone()).collect()
    }

    pub fn remove_policy(&self, app_id: &AppId) -> Result<()> {
        self.policies
            .remove(app_id)
            .ok_or_else(|| SecurityError::AccessDenied(app_id.to_string()))?;

        Ok(())
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_policy() {
        let manager = PermissionManager::new();
        let policy = SecurityPolicy {
            app_id: AppId::new("test").unwrap(),
            permissions: HashSet::new(),
            resource_limits: ResourceLimits {
                cpu_percent: 50,
                memory_mb: 512,
                disk_quota_mb: 1024,
                network_bandwidth_mbps: 100,
            },
            sandbox_level: SandboxLevel::Basic,
            trusted: false,
        };

        manager.register_policy(policy).unwrap();
        let app_id = AppId::new("test").unwrap();
        assert!(manager.get_policy(&app_id).is_ok());
    }

    #[test]
    fn test_grant_permission() {
        let manager = PermissionManager::new();
        let app_id = AppId::new("test").unwrap();
        let policy = SecurityPolicy {
            app_id: app_id.clone(),
            permissions: HashSet::new(),
            resource_limits: ResourceLimits {
                cpu_percent: 50,
                memory_mb: 512,
                disk_quota_mb: 1024,
                network_bandwidth_mbps: 100,
            },
            sandbox_level: SandboxLevel::Basic,
            trusted: false,
        };

        manager.register_policy(policy).unwrap();
        manager.grant_permission(&app_id, Permission::FilesystemRead).unwrap();
        assert!(manager.has_permission(&app_id, Permission::FilesystemRead).unwrap());
    }

    #[test]
    fn test_revoke_permission() {
        let manager = PermissionManager::new();
        let app_id = AppId::new("test").unwrap();
        let mut permissions = HashSet::new();
        permissions.insert(Permission::FilesystemRead);

        let policy = SecurityPolicy {
            app_id: app_id.clone(),
            permissions,
            resource_limits: ResourceLimits {
                cpu_percent: 50,
                memory_mb: 512,
                disk_quota_mb: 1024,
                network_bandwidth_mbps: 100,
            },
            sandbox_level: SandboxLevel::Basic,
            trusted: false,
        };

        manager.register_policy(policy).unwrap();
        manager.revoke_permission(&app_id, Permission::FilesystemRead).unwrap();
        assert!(!manager.has_permission(&app_id, Permission::FilesystemRead).unwrap());
    }
}
