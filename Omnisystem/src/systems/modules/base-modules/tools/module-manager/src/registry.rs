//! Package registry for multi-language module discovery

use crate::{ModuleId, ModuleMetadata, ModuleManagerError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Package metadata in registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    pub id: ModuleId,
    pub name: String,
    pub language: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub documentation: Option<String>,
    pub license: String,
    pub keywords: Vec<String>,
    pub downloads: u64,
    pub rating: f32,
    pub review_count: u32,
}

/// Registry search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySearchResult {
    pub query: String,
    pub results: Vec<PackageMetadata>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
}

/// Language-agnostic package registry trait
pub trait PackageRegistry: Send + Sync {
    /// Get module metadata
    fn get_metadata(&self, id: &ModuleId) -> Result<ModuleMetadata>;

    /// Get package metadata
    fn get_package(&self, name: &str) -> Result<PackageMetadata>;

    /// Search for packages
    fn search(
        &self,
        query: &str,
        language: Option<&str>,
        page: u32,
        per_page: u32,
    ) -> Result<RegistrySearchResult>;

    /// Download module to path
    fn download_module(&self, id: &ModuleId, path: &Path) -> Result<()>;

    /// List by language
    fn list_by_language(&self, language: &str) -> Result<Vec<PackageMetadata>>;

    /// Resolve version (returns concrete version for requirement)
    fn resolve_version(&self, name: &str, requirement: &str) -> Result<String>;
}

/// In-memory package registry (default implementation)
pub struct InMemoryRegistry {
    packages: HashMap<String, PackageMetadata>,
    modules: HashMap<String, ModuleMetadata>,
}

impl InMemoryRegistry {
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
            modules: HashMap::new(),
        }
    }

    /// Add package to registry
    pub fn add_package(&mut self, package: PackageMetadata) {
        self.packages.insert(package.id.full_id(), package);
    }

    /// Add module metadata
    pub fn add_module(&mut self, metadata: ModuleMetadata) {
        self.modules.insert(metadata.id.full_id(), metadata);
    }

    /// Get all packages
    pub fn list(&self) -> Vec<PackageMetadata> {
        self.packages.values().cloned().collect()
    }
}

impl Default for InMemoryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PackageRegistry for InMemoryRegistry {
    fn get_metadata(&self, id: &ModuleId) -> Result<ModuleMetadata> {
        self.modules.get(&id.full_id()).cloned().ok_or_else(|| {
            ModuleManagerError::ModuleNotFound(id.full_id())
        })
    }

    fn get_package(&self, name: &str) -> Result<PackageMetadata> {
        self.packages
            .values()
            .find(|p| p.name == name)
            .cloned()
            .ok_or_else(|| ModuleManagerError::ModuleNotFound(name.to_string()))
    }

    fn search(
        &self,
        query: &str,
        language: Option<&str>,
        page: u32,
        per_page: u32,
    ) -> Result<RegistrySearchResult> {
        let mut results: Vec<PackageMetadata> = self
            .packages
            .values()
            .filter(|p| {
                let matches_query = p.name.to_lowercase().contains(&query.to_lowercase())
                    || p.description.to_lowercase().contains(&query.to_lowercase())
                    || p.keywords.iter().any(|k| k.to_lowercase().contains(&query.to_lowercase()));

                let matches_language = language.is_none() || p.language == language.unwrap();

                matches_query && matches_language
            })
            .cloned()
            .collect();

        results.sort_by(|a, b| b.downloads.cmp(&a.downloads));

        let total = results.len() as u64;
        let skip = ((page - 1) * per_page) as usize;
        let results: Vec<PackageMetadata> = results
            .into_iter()
            .skip(skip)
            .take(per_page as usize)
            .collect();

        Ok(RegistrySearchResult {
            query: query.to_string(),
            total,
            results,
            page,
            per_page,
        })
    }

    fn download_module(&self, id: &ModuleId, path: &Path) -> Result<()> {
        // Stub: real implementation would fetch from remote registry
        std::fs::create_dir_all(path).map_err(|e| {
            ModuleManagerError::LoadingFailed(format!("Failed to create module dir: {}", e))
        })?;
        log::info!("Downloaded module {} to {:?}", id.full_id(), path);
        Ok(())
    }

    fn list_by_language(&self, language: &str) -> Result<Vec<PackageMetadata>> {
        let results: Vec<PackageMetadata> = self
            .packages
            .values()
            .filter(|p| p.language == language)
            .cloned()
            .collect();
        Ok(results)
    }

    fn resolve_version(&self, _name: &str, requirement: &str) -> Result<String> {
        // Stub: real implementation would parse semver requirements
        // For now, return requirement as-is
        Ok(requirement.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = InMemoryRegistry::new();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_registry_add_and_get() {
        let mut registry = InMemoryRegistry::new();
        let id = ModuleId::new("omnisystem", "compiler", "1.0.0");
        let package = PackageMetadata {
            id: id.clone(),
            name: "compiler".to_string(),
            language: "rust".to_string(),
            version: "1.0.0".to_string(),
            description: "Universal Compiler".to_string(),
            author: "Omnisystem Team".to_string(),
            homepage: None,
            repository: None,
            documentation: None,
            license: "Apache-2.0".to_string(),
            keywords: vec!["compiler".to_string()],
            downloads: 0,
            rating: 5.0,
            review_count: 0,
        };

        registry.add_package(package.clone());
        let retrieved = registry.get_package("compiler").unwrap();
        assert_eq!(retrieved.name, "compiler");
    }

    #[test]
    fn test_registry_search() {
        let mut registry = InMemoryRegistry::new();
        let id = ModuleId::new("omnisystem", "compiler", "1.0.0");
        let package = PackageMetadata {
            id,
            name: "compiler".to_string(),
            language: "rust".to_string(),
            version: "1.0.0".to_string(),
            description: "Universal Compiler".to_string(),
            author: "Omnisystem Team".to_string(),
            homepage: None,
            repository: None,
            documentation: None,
            license: "Apache-2.0".to_string(),
            keywords: vec!["compiler".to_string()],
            downloads: 100,
            rating: 5.0,
            review_count: 0,
        };

        registry.add_package(package);
        let result = registry.search("compiler", None, 1, 10).unwrap();
        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].name, "compiler");
    }

    #[test]
    fn test_registry_search_by_language() {
        let mut registry = InMemoryRegistry::new();

        let rust_module = PackageMetadata {
            id: ModuleId::new("omnisystem", "compiler", "1.0.0"),
            name: "compiler".to_string(),
            language: "rust".to_string(),
            version: "1.0.0".to_string(),
            description: "Compiler".to_string(),
            author: "Team".to_string(),
            homepage: None,
            repository: None,
            documentation: None,
            license: "MIT".to_string(),
            keywords: vec![],
            downloads: 0,
            rating: 5.0,
            review_count: 0,
        };

        let python_module = PackageMetadata {
            id: ModuleId::new("omnisystem", "analyzer", "1.0.0"),
            name: "analyzer".to_string(),
            language: "python".to_string(),
            version: "1.0.0".to_string(),
            description: "Analyzer".to_string(),
            author: "Team".to_string(),
            homepage: None,
            repository: None,
            documentation: None,
            license: "MIT".to_string(),
            keywords: vec![],
            downloads: 0,
            rating: 5.0,
            review_count: 0,
        };

        registry.add_package(rust_module);
        registry.add_package(python_module);

        let rust_results = registry.list_by_language("rust").unwrap();
        assert_eq!(rust_results.len(), 1);
        assert_eq!(rust_results[0].language, "rust");

        let python_results = registry.list_by_language("python").unwrap();
        assert_eq!(python_results.len(), 1);
        assert_eq!(python_results[0].language, "python");
    }
}
