use crate::{AppId, Version, Result};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMetadata {
    pub app_id: AppId,
    pub name: String,
    pub version: Version,
    pub description: String,
    pub author: String,
    pub homepage: String,
    pub repository: String,
    pub license: String,

    pub omnisystem_version: String,
    pub min_memory_mb: u32,
    pub min_disk_mb: u32,
    pub requires_gpu: bool,

    pub total_downloads: u64,
    pub rating: f32,
    pub stars: u32,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub installed_at: Option<DateTime<Utc>>,

    pub signature: String,
    pub checksum: String,
    pub is_signed: bool,
    pub is_official: bool,

    pub tags: Vec<String>,
    pub keywords: Vec<String>,
}

impl AppMetadata {
    pub fn new(
        app_id: AppId,
        name: String,
        version: Version,
        author: String,
        license: String,
    ) -> Self {
        let now = Utc::now();
        AppMetadata {
            app_id,
            name,
            version,
            description: String::new(),
            author,
            homepage: String::new(),
            repository: String::new(),
            license,

            omnisystem_version: "1.0.0".to_string(),
            min_memory_mb: 256,
            min_disk_mb: 100,
            requires_gpu: false,

            total_downloads: 0,
            rating: 0.0,
            stars: 0,

            created_at: now,
            updated_at: now,
            installed_at: None,

            signature: String::new(),
            checksum: String::new(),
            is_signed: false,
            is_official: false,

            tags: Vec::new(),
            keywords: Vec::new(),
        }
    }

    pub fn set_description(&mut self, desc: String) -> &mut Self {
        self.description = desc;
        self
    }

    pub fn set_homepage(&mut self, home: String) -> &mut Self {
        self.homepage = home;
        self
    }

    pub fn set_repository(&mut self, repo: String) -> &mut Self {
        self.repository = repo;
        self
    }

    pub fn set_resources(&mut self, memory_mb: u32, disk_mb: u32) -> &mut Self {
        self.min_memory_mb = memory_mb;
        self.min_disk_mb = disk_mb;
        self
    }

    pub fn set_gpu_required(&mut self, required: bool) -> &mut Self {
        self.requires_gpu = required;
        self
    }

    pub fn add_tag(&mut self, tag: String) -> &mut Self {
        self.tags.push(tag);
        self
    }

    pub fn add_keyword(&mut self, keyword: String) -> &mut Self {
        self.keywords.push(keyword);
        self
    }

    pub fn mark_installed(&mut self) {
        self.installed_at = Some(Utc::now());
    }

    pub fn mark_official(&mut self) {
        self.is_official = true;
    }

    pub fn is_compatible(&self) -> bool {
        true
    }

    pub fn meets_requirements(&self, available_memory_mb: u32, available_disk_mb: u32) -> bool {
        available_memory_mb >= self.min_memory_mb && available_disk_mb >= self.min_disk_mb
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self)
            .map_err(|e| crate::AppManagerError::SerializationError(e.to_string()))
    }

    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| crate::AppManagerError::SerializationError(e.to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataCache {
    entries: HashMap<AppId, AppMetadata>,
}

impl MetadataCache {
    pub fn new() -> Self {
        MetadataCache {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, metadata: AppMetadata) {
        self.entries.insert(metadata.app_id.clone(), metadata);
    }

    pub fn get(&self, app_id: &AppId) -> Option<AppMetadata> {
        self.entries.get(app_id).cloned()
    }

    pub fn remove(&mut self, app_id: &AppId) -> Option<AppMetadata> {
        self.entries.remove(app_id)
    }

    pub fn list_all(&self) -> Vec<AppMetadata> {
        self.entries.values().cloned().collect()
    }

    pub fn find_by_name(&self, name: &str) -> Vec<AppMetadata> {
        self.entries
            .values()
            .filter(|m| m.name.contains(name))
            .cloned()
            .collect()
    }

    pub fn find_by_tag(&self, tag: &str) -> Vec<AppMetadata> {
        self.entries
            .values()
            .filter(|m| m.tags.contains(&tag.to_string()))
            .cloned()
            .collect()
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl Default for MetadataCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_metadata_creation() {
        let metadata = AppMetadata::new(
            AppId::new("test-app").unwrap(),
            "Test App".to_string(),
            Version::new(1, 0, 0),
            "Author".to_string(),
            "MIT".to_string(),
        );

        assert_eq!(metadata.name, "Test App");
        assert_eq!(metadata.author, "Author");
        assert_eq!(metadata.license, "MIT");
    }

    #[test]
    fn test_app_metadata_builder() {
        let mut metadata = AppMetadata::new(
            AppId::new("test-app").unwrap(),
            "Test App".to_string(),
            Version::new(1, 0, 0),
            "Author".to_string(),
            "MIT".to_string(),
        );

        metadata
            .set_description("A test app".to_string())
            .set_resources(512, 200)
            .add_tag("testing".to_string());

        assert_eq!(metadata.description, "A test app");
        assert_eq!(metadata.min_memory_mb, 512);
        assert_eq!(metadata.min_disk_mb, 200);
        assert!(metadata.tags.contains(&"testing".to_string()));
    }

    #[test]
    fn test_metadata_cache() {
        let mut cache = MetadataCache::new();
        let metadata = AppMetadata::new(
            AppId::new("test-app").unwrap(),
            "Test App".to_string(),
            Version::new(1, 0, 0),
            "Author".to_string(),
            "MIT".to_string(),
        );

        cache.insert(metadata.clone());
        assert_eq!(cache.count(), 1);

        let retrieved = cache.get(&metadata.app_id).unwrap();
        assert_eq!(retrieved.name, "Test App");

        cache.remove(&metadata.app_id);
        assert_eq!(cache.count(), 0);
    }

    #[test]
    fn test_meets_requirements() {
        let mut metadata = AppMetadata::new(
            AppId::new("test-app").unwrap(),
            "Test App".to_string(),
            Version::new(1, 0, 0),
            "Author".to_string(),
            "MIT".to_string(),
        );

        metadata.set_resources(512, 200);

        assert!(metadata.meets_requirements(1024, 400));
        assert!(!metadata.meets_requirements(256, 400));
        assert!(!metadata.meets_requirements(1024, 100));
    }

    #[test]
    fn test_metadata_serialization() {
        let metadata = AppMetadata::new(
            AppId::new("test-app").unwrap(),
            "Test App".to_string(),
            Version::new(1, 0, 0),
            "Author".to_string(),
            "MIT".to_string(),
        );

        let json = metadata.to_json().unwrap();
        let deserialized = AppMetadata::from_json(&json).unwrap();

        assert_eq!(deserialized.name, metadata.name);
        assert_eq!(deserialized.version, metadata.version);
    }
}
