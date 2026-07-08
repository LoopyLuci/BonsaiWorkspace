//! Application model and manifest types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::error::AppManagerResult;

/// Unique identifier for an application
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct AppId(pub Uuid);

impl AppId {
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

impl Default for AppId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AppId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a publisher
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PublisherId(pub Uuid);

impl PublisherId {
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

impl Default for PublisherId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PublisherId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Complete application manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppManifest {
    pub id: AppId,
    pub name: String,
    pub version: semver::Version,
    pub description: String,
    pub publisher_id: PublisherId,
    pub license: String,

    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub languages: Vec<String>,
    pub platforms: Vec<String>,

    pub icon_url: String,
    pub screenshots: Vec<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    pub min_omnisystem_version: semver::Version,
    pub required_memory_mb: u32,
    pub required_disk_mb: u32,

    pub metadata: HashMap<String, serde_json::Value>,
}

impl AppManifest {
    pub fn new(
        name: String,
        version: semver::Version,
        publisher_id: PublisherId,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: AppId::new(),
            name,
            version,
            description: String::new(),
            publisher_id,
            license: "MIT".to_string(),
            categories: vec![],
            tags: vec![],
            languages: vec!["en".to_string()],
            platforms: vec![],
            icon_url: String::new(),
            screenshots: vec![],
            homepage: None,
            repository: None,
            created_at: now,
            updated_at: now,
            min_omnisystem_version: semver::Version::new(1, 0, 0),
            required_memory_mb: 256,
            required_disk_mb: 100,
            metadata: HashMap::new(),
        }
    }

    pub fn validate(&self) -> AppManagerResult<()> {
        use crate::error::AppManagerError;

        if self.name.is_empty() {
            return Err(AppManagerError::InvalidManifest(
                "App name cannot be empty".into(),
            ));
        }

        if self.version.major == 0 && self.version.minor == 0 && self.version.patch == 0 {
            return Err(AppManagerError::InvalidManifest(
                "Invalid version 0.0.0".into(),
            ));
        }

        if self.icon_url.is_empty() {
            return Err(AppManagerError::InvalidManifest(
                "Icon URL cannot be empty".into(),
            ));
        }

        Ok(())
    }

    pub fn from_json(json: &str) -> AppManagerResult<Self> {
        serde_json::from_str(json).map_err(crate::error::AppManagerError::JsonError)
    }

    pub fn to_json(&self) -> AppManagerResult<String> {
        serde_json::to_string_pretty(self).map_err(crate::error::AppManagerError::JsonError)
    }
}

/// Installed application with runtime information
#[derive(Debug, Clone)]
pub struct RegisteredApp {
    pub manifest: AppManifest,
    pub installed: bool,
    pub installed_at: Option<DateTime<Utc>>,
    pub location: Option<std::path::PathBuf>,
    pub rating: f32,
    pub review_count: u32,
    pub download_count: u32,
}

impl RegisteredApp {
    pub fn new(manifest: AppManifest) -> Self {
        Self {
            manifest,
            installed: false,
            installed_at: None,
            location: None,
            rating: 0.0,
            review_count: 0,
            download_count: 0,
        }
    }

    pub fn mark_installed(&mut self, path: std::path::PathBuf) {
        self.installed = true;
        self.installed_at = Some(Utc::now());
        self.location = Some(path);
    }

    pub fn mark_uninstalled(&mut self) {
        self.installed = false;
        self.installed_at = None;
        self.location = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_id_generation() {
        let id1 = AppId::new();
        let id2 = AppId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_app_manifest_validation() {
        let mut manifest = AppManifest::new(
            "Test App".to_string(),
            semver::Version::new(1, 0, 0),
            PublisherId::new(),
        );
        manifest.icon_url = "icon.png".to_string();
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_app_manifest_validation_fails_empty_name() {
        let manifest = AppManifest::new(
            String::new(),
            semver::Version::new(1, 0, 0),
            PublisherId::new(),
        );
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_app_manifest_json_serialization() {
        let mut manifest = AppManifest::new(
            "Test App".to_string(),
            semver::Version::new(1, 0, 0),
            PublisherId::new(),
        );
        manifest.icon_url = "icon.png".to_string();

        let json = manifest.to_json().unwrap();
        let deserialized = AppManifest::from_json(&json).unwrap();
        assert_eq!(manifest.id, deserialized.id);
        assert_eq!(manifest.name, deserialized.name);
    }

    #[test]
    fn test_registered_app_lifecycle() {
        let manifest = AppManifest::new(
            "Test App".to_string(),
            semver::Version::new(1, 0, 0),
            PublisherId::new(),
        );
        let mut app = RegisteredApp::new(manifest);

        assert!(!app.installed);
        assert!(app.installed_at.is_none());

        app.mark_installed(std::path::PathBuf::from("/apps/test"));
        assert!(app.installed);
        assert!(app.installed_at.is_some());
        assert_eq!(app.location, Some(std::path::PathBuf::from("/apps/test")));

        app.mark_uninstalled();
        assert!(!app.installed);
        assert!(app.installed_at.is_none());
    }
}
