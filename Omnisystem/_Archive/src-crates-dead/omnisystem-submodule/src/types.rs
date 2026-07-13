use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleState {
    Unloaded,
    Loading,
    Initialized,
    Started,
    Running,
    Paused,
    Stopping,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyMode {
    Required,
    Optional,
    Transitive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ModuleVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    pub fn matches_required(&self, required: &ModuleVersion) -> bool {
        self.major == required.major && self.minor >= required.minor
    }
}

impl std::fmt::Display for ModuleVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDependency {
    pub name: String,
    pub version: ModuleVersion,
    pub mode: DependencyMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetadata {
    pub name: String,
    pub version: ModuleVersion,
    pub author: String,
    pub description: String,
    pub dependencies: Vec<ModuleDependency>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotReloadConfig {
    pub enabled: bool,
    pub preserve_state: bool,
    pub max_attempts: u32,
    pub timeout_ms: u64,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            preserve_state: true,
            max_attempts: 3,
            timeout_ms: 5000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_version_matches() {
        let v1 = ModuleVersion::new(1, 2, 3);
        let v2 = ModuleVersion::new(1, 2, 0);
        assert!(v1.matches_required(&v2));
    }

    #[test]
    fn test_module_state_enum() {
        assert_eq!(ModuleState::Running, ModuleState::Running);
        assert_ne!(ModuleState::Running, ModuleState::Stopped);
    }

    #[test]
    fn test_metadata_creation() {
        let metadata = ModuleMetadata {
            name: "test".to_string(),
            version: ModuleVersion::new(1, 0, 0),
            author: "test".to_string(),
            description: "test".to_string(),
            dependencies: vec![],
            capabilities: vec!["feature1".to_string()],
        };
        assert_eq!(metadata.name, "test");
    }

    #[test]
    fn test_dependency_mode() {
        assert_eq!(DependencyMode::Required, DependencyMode::Required);
    }
}
