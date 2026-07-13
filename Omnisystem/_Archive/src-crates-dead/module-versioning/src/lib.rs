use dashmap::DashMap;
use module_interfaces::{ModuleError, ModuleVersion, VersionConstraint, VersionConstraintType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleVersionInfo {
    pub module_id: String,
    pub version: ModuleVersion,
    pub release_date: u64,
    pub deprecated: bool,
    pub breaking_changes: Vec<String>,
    pub features: Vec<String>,
    pub bugfixes: Vec<String>,
    pub compatibility_matrix: Vec<CompatibilityInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompatibilityInfo {
    pub dependency_id: String,
    pub min_version: String,
    pub max_version: Option<String>,
}

pub struct VersionManager {
    versions: Arc<DashMap<String, Vec<ModuleVersionInfo>>>,
    installed_versions: Arc<DashMap<String, ModuleVersion>>,
    upgrade_paths: Arc<DashMap<String, Vec<UpgradePath>>>,
}

#[derive(Clone, Debug)]
pub struct UpgradePath {
    pub from_version: ModuleVersion,
    pub to_version: ModuleVersion,
    pub steps: Vec<String>,
    pub requires_downtime: bool,
    pub estimated_duration_seconds: u32,
}

impl VersionManager {
    pub fn new() -> Self {
        info!("Creating VersionManager");
        Self {
            versions: Arc::new(DashMap::new()),
            installed_versions: Arc::new(DashMap::new()),
            upgrade_paths: Arc::new(DashMap::new()),
        }
    }

    pub fn register_version(&self, version_info: ModuleVersionInfo) -> Result<(), ModuleError> {
        debug!("Registering version for module: {} v{}", version_info.module_id, version_info.version.to_string_canonical());

        self.versions
            .entry(version_info.module_id.clone())
            .or_insert_with(Vec::new)
            .push(version_info);

        Ok(())
    }

    pub fn get_latest_version(&self, module_id: &str) -> Result<ModuleVersionInfo, ModuleError> {
        debug!("Getting latest version for module: {}", module_id);

        self.versions
            .get(module_id)
            .and_then(|versions| versions.iter().max_by_key(|v| (v.version.major, v.version.minor, v.version.patch)).cloned())
            .ok_or_else(|| ModuleError::NotFound(module_id.to_string()))
    }

    pub fn check_compatibility(&self, module_id: &str, version: &ModuleVersion, dependency: &str, dep_version: &ModuleVersion) -> Result<bool, ModuleError> {
        debug!("Checking compatibility: {} v{} with {} v{}", module_id, version.to_string_canonical(), dependency, dep_version.to_string_canonical());

        match self.versions.get(module_id) {
            Some(versions) => {
                let module_ver = versions.iter().find(|v| v.version == *version);
                match module_ver {
                    Some(info) => {
                        let compatible = info
                            .compatibility_matrix
                            .iter()
                            .filter(|c| c.dependency_id == dependency)
                            .any(|c| {
                                let min_ok = ModuleVersion::parse(&c.min_version)
                                    .ok()
                                    .map(|min| dep_version >= &min)
                                    .unwrap_or(false);

                                let max_ok = c.max_version.as_ref().map(|max| ModuleVersion::parse(max).ok().map(|m| dep_version <= &m).unwrap_or(false)).unwrap_or(true);

                                min_ok && max_ok
                            });

                        Ok(compatible)
                    }
                    None => Ok(false),
                }
            }
            None => Ok(false),
        }
    }

    pub fn plan_upgrade(&self, module_id: &str, from: &ModuleVersion, to: &ModuleVersion) -> Result<UpgradePath, ModuleError> {
        debug!("Planning upgrade for module: {} from {} to {}", module_id, from.to_string_canonical(), to.to_string_canonical());

        let path = UpgradePath {
            from_version: from.clone(),
            to_version: to.clone(),
            steps: vec!["Prepare backup".to_string(), "Stop module".to_string(), "Update module".to_string(), "Restart module".to_string(), "Verify functionality".to_string()],
            requires_downtime: from.major != to.major,
            estimated_duration_seconds: if from.major != to.major { 300 } else { 30 },
        };

        Ok(path)
    }

    pub fn set_installed_version(&self, module_id: String, version: ModuleVersion) -> Result<(), ModuleError> {
        debug!("Setting installed version for module: {} to v{}", module_id, version.to_string_canonical());
        self.installed_versions.insert(module_id, version);
        Ok(())
    }

    pub fn get_installed_version(&self, module_id: &str) -> Result<ModuleVersion, ModuleError> {
        self.installed_versions
            .get(module_id)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| ModuleError::NotFound(module_id.to_string()))
    }

    pub fn list_all_versions(&self, module_id: &str) -> Result<Vec<ModuleVersionInfo>, ModuleError> {
        self.versions
            .get(module_id)
            .map(|versions| versions.iter().cloned().collect())
            .ok_or_else(|| ModuleError::NotFound(module_id.to_string()))
    }
}

impl Default for VersionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for VersionManager {
    fn clone(&self) -> Self {
        Self {
            versions: Arc::clone(&self.versions),
            installed_versions: Arc::clone(&self.installed_versions),
            upgrade_paths: Arc::clone(&self.upgrade_paths),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_manager_creation() {
        let manager = VersionManager::new();
        assert_eq!(manager.installed_versions.len(), 0);
    }

    #[test]
    fn test_register_version() {
        let manager = VersionManager::new();
        let version_info = ModuleVersionInfo {
            module_id: "test-module".to_string(),
            version: ModuleVersion {
                major: 1,
                minor: 0,
                patch: 0,
                pre_release: None,
                build: None,
            },
            release_date: 0,
            deprecated: false,
            breaking_changes: vec![],
            features: vec!["feature1".to_string()],
            bugfixes: vec![],
            compatibility_matrix: vec![],
        };

        assert!(manager.register_version(version_info).is_ok());
    }

    #[test]
    fn test_set_installed_version() {
        let manager = VersionManager::new();
        let version = ModuleVersion {
            major: 1,
            minor: 0,
            patch: 0,
            pre_release: None,
            build: None,
        };

        assert!(manager.set_installed_version("test-module".to_string(), version).is_ok());
    }
}
