//! Module model and lifecycle management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::app::AppId;
use crate::error::AppManagerResult;

/// Unique identifier for a module
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ModuleId(pub Uuid);

impl ModuleId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for ModuleId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type of module
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum ModuleType {
    Library,
    Service,
    Widget,
    Plugin,
    Driver,
    Utility,
}

impl std::fmt::Display for ModuleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleType::Library => write!(f, "library"),
            ModuleType::Service => write!(f, "service"),
            ModuleType::Widget => write!(f, "widget"),
            ModuleType::Plugin => write!(f, "plugin"),
            ModuleType::Driver => write!(f, "driver"),
            ModuleType::Utility => write!(f, "utility"),
        }
    }
}

/// Current status of a module
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum ModuleStatus {
    Discovered,
    Registered,
    Loading,
    Loaded,
    Failed(String),
    Unloaded,
}

impl std::fmt::Display for ModuleStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleStatus::Discovered => write!(f, "discovered"),
            ModuleStatus::Registered => write!(f, "registered"),
            ModuleStatus::Loading => write!(f, "loading"),
            ModuleStatus::Loaded => write!(f, "loaded"),
            ModuleStatus::Failed(reason) => write!(f, "failed: {}", reason),
            ModuleStatus::Unloaded => write!(f, "unloaded"),
        }
    }
}

/// Complete module manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleManifest {
    pub id: ModuleId,
    pub app_id: AppId,
    pub name: String,
    pub version: semver::Version,
    pub module_type: ModuleType,

    pub entry_points: HashMap<String, String>,
    pub exported_symbols: Vec<String>,
    pub dependencies: Vec<crate::dependency::ModuleDependency>,
    pub permissions: Vec<String>,

    pub file_hash: String,
    pub file_size: u64,
    pub source_code_available: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    pub metadata: HashMap<String, serde_json::Value>,
}

impl ModuleManifest {
    pub fn new(
        app_id: AppId,
        name: String,
        version: semver::Version,
        module_type: ModuleType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: ModuleId::new(),
            app_id,
            name,
            version,
            module_type,
            entry_points: HashMap::new(),
            exported_symbols: vec![],
            dependencies: vec![],
            permissions: vec![],
            file_hash: String::new(),
            file_size: 0,
            source_code_available: false,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    pub fn validate(&self) -> AppManagerResult<()> {
        use crate::error::AppManagerError;

        if self.name.is_empty() {
            return Err(AppManagerError::InvalidManifest(
                "Module name cannot be empty".into(),
            ));
        }

        if self.entry_points.is_empty() {
            return Err(AppManagerError::InvalidManifest(
                "Module must have at least one entry point".into(),
            ));
        }

        if self.file_hash.is_empty() {
            return Err(AppManagerError::InvalidManifest(
                "File hash cannot be empty".into(),
            ));
        }

        Ok(())
    }

    pub fn add_entry_point(&mut self, name: String, path: String) {
        self.entry_points.insert(name, path);
        self.updated_at = Utc::now();
    }

    pub fn add_dependency(&mut self, dep: crate::dependency::ModuleDependency) {
        self.dependencies.push(dep);
        self.updated_at = Utc::now();
    }

    pub fn from_json(json: &str) -> AppManagerResult<Self> {
        serde_json::from_str(json).map_err(crate::error::AppManagerError::JsonError)
    }

    pub fn to_json(&self) -> AppManagerResult<String> {
        serde_json::to_string_pretty(self).map_err(crate::error::AppManagerError::JsonError)
    }
}

/// Module instance in registry with runtime state
#[derive(Debug, Clone)]
pub struct RegisteredModule {
    pub manifest: ModuleManifest,
    pub status: ModuleStatus,
    pub location: Option<std::path::PathBuf>,
    pub loaded_at: Option<DateTime<Utc>>,
    pub memory_usage_bytes: u64,
}

impl RegisteredModule {
    pub fn new(manifest: ModuleManifest) -> Self {
        Self {
            manifest,
            status: ModuleStatus::Discovered,
            location: None,
            loaded_at: None,
            memory_usage_bytes: 0,
        }
    }

    pub fn mark_registered(&mut self) {
        self.status = ModuleStatus::Registered;
    }

    pub fn mark_loading(&mut self) {
        self.status = ModuleStatus::Loading;
    }

    pub fn mark_loaded(&mut self, location: std::path::PathBuf) {
        self.status = ModuleStatus::Loaded;
        self.location = Some(location);
        self.loaded_at = Some(Utc::now());
    }

    pub fn mark_failed(&mut self, reason: String) {
        self.status = ModuleStatus::Failed(reason);
    }

    pub fn mark_unloaded(&mut self) {
        self.status = ModuleStatus::Unloaded;
        self.loaded_at = None;
    }

    pub fn is_loaded(&self) -> bool {
        matches!(self.status, ModuleStatus::Loaded)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self.status, ModuleStatus::Failed(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_id_generation() {
        let id1 = ModuleId::new();
        let id2 = ModuleId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_module_manifest_creation() {
        let app_id = AppId::new();
        let manifest = ModuleManifest::new(
            app_id.clone(),
            "test-module".to_string(),
            semver::Version::new(1, 0, 0),
            ModuleType::Library,
        );

        assert_eq!(manifest.app_id, app_id);
        assert_eq!(manifest.name, "test-module");
        assert_eq!(manifest.module_type, ModuleType::Library);
    }

    #[test]
    fn test_module_manifest_validation() {
        let app_id = AppId::new();
        let mut manifest = ModuleManifest::new(
            app_id,
            "test".to_string(),
            semver::Version::new(1, 0, 0),
            ModuleType::Service,
        );

        assert!(manifest.validate().is_err()); // No entry points

        manifest.add_entry_point("main".to_string(), "/module/main.so".to_string());
        manifest.file_hash = "abc123".to_string();

        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_registered_module_lifecycle() {
        let app_id = AppId::new();
        let manifest = ModuleManifest::new(
            app_id,
            "test".to_string(),
            semver::Version::new(1, 0, 0),
            ModuleType::Library,
        );
        let mut module = RegisteredModule::new(manifest);

        assert_eq!(module.status, ModuleStatus::Discovered);
        assert!(!module.is_loaded());

        module.mark_registered();
        assert_eq!(module.status, ModuleStatus::Registered);

        module.mark_loading();
        assert_eq!(module.status, ModuleStatus::Loading);

        module.mark_loaded(std::path::PathBuf::from("/modules/test"));
        assert!(module.is_loaded());
        assert_eq!(module.location, Some(std::path::PathBuf::from("/modules/test")));

        module.mark_unloaded();
        assert!(!module.is_loaded());
        assert_eq!(module.status, ModuleStatus::Unloaded);
    }

    #[test]
    fn test_module_status_display() {
        assert_eq!(ModuleStatus::Loaded.to_string(), "loaded");
        assert_eq!(ModuleStatus::Failed("test error".to_string()).to_string(), "failed: test error");
    }
}
