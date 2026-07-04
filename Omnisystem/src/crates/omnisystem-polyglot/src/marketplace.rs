/// OMNISYSTEM POLYGLOT: MODULE MARKETPLACE
/// Discover, publish, and manage polyglot modules across all 750+ languages
/// Enables community sharing of reusable language modules

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Module Package Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModulePackage {
    /// Package identifier (namespace/name)
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Supported languages (e.g., ["rust", "python", "javascript"])
    pub languages: Vec<String>,
    /// Package version (semver)
    pub version: String,
    /// Author
    pub author: String,
    /// Repository URL
    pub repository: Option<String>,
    /// License (e.g., MIT, Apache-2.0)
    pub license: String,
    /// Module documentation URL
    pub docs_url: Option<String>,
    /// Download count
    pub downloads: u64,
    /// Rating (0-5 stars)
    pub rating: f32,
    /// Dependencies on other polyglot modules
    pub dependencies: Vec<ModuleDependency>,
    /// Timestamp of publication
    pub published_at: String,
    /// Module size in bytes
    pub size_bytes: usize,
}

/// Module Dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDependency {
    pub package_id: String,
    pub version_requirement: String, // e.g., "^1.0.0", ">=2.0.0"
}

/// Module Release
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleRelease {
    pub package_id: String,
    pub version: String,
    pub release_notes: String,
    pub file_hash: String, // SHA256 hash for integrity
    pub published_at: String,
    pub yanked: bool, // If true, don't recommend this version
}

/// Marketplace Registry - manages all published modules
pub struct ModuleMarketplace {
    packages: Arc<DashMap<String, ModulePackage>>,
    releases: Arc<DashMap<String, Vec<ModuleRelease>>>, // package_id -> versions
    categories: Arc<DashMap<String, Vec<String>>>,       // category -> package_ids
    stats: Arc<parking_lot::Mutex<MarketplaceStats>>,
}

/// Marketplace Statistics
#[derive(Debug, Clone)]
pub struct MarketplaceStats {
    pub total_packages: usize,
    pub total_downloads: u64,
    pub total_languages_supported: usize,
    pub featured_packages: Vec<String>,
}

impl ModuleMarketplace {
    pub fn new() -> Self {
        ModuleMarketplace {
            packages: Arc::new(DashMap::new()),
            releases: Arc::new(DashMap::new()),
            categories: Arc::new(DashMap::new()),
            stats: Arc::new(parking_lot::Mutex::new(MarketplaceStats {
                total_packages: 0,
                total_downloads: 0,
                total_languages_supported: 0,
                featured_packages: Vec::new(),
            })),
        }
    }

    /// Publish a new module package
    pub fn publish_package(&self, package: ModulePackage) -> Result<(), String> {
        if self.packages.contains_key(&package.id) {
            return Err(format!("Package {} already exists", package.id));
        }

        self.packages.insert(package.id.clone(), package.clone());

        // Update stats
        let mut stats = self.stats.lock();
        stats.total_packages += 1;

        Ok(())
    }

    /// Release a new version of a module
    pub fn release_version(
        &self,
        package_id: &str,
        release: ModuleRelease,
    ) -> Result<(), String> {
        if !self.packages.contains_key(package_id) {
            return Err(format!("Package {} not found", package_id));
        }

        self.releases
            .entry(package_id.to_string())
            .or_insert_with(Vec::new)
            .push(release);

        Ok(())
    }

    /// Get a package by ID
    pub fn get_package(&self, package_id: &str) -> Option<ModulePackage> {
        self.packages.get(package_id).map(|p| p.clone())
    }

    /// Search packages by language
    pub fn search_by_language(&self, language: &str) -> Vec<ModulePackage> {
        self.packages
            .iter()
            .filter(|entry| entry.value().languages.contains(&language.to_string()))
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Search packages by name/description
    pub fn search_by_query(&self, query: &str) -> Vec<ModulePackage> {
        let query_lower = query.to_lowercase();
        self.packages
            .iter()
            .filter(|entry| {
                let pkg = entry.value();
                pkg.name.to_lowercase().contains(&query_lower)
                    || pkg.description.to_lowercase().contains(&query_lower)
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get top rated packages
    pub fn get_top_rated(&self, limit: usize) -> Vec<ModulePackage> {
        let mut packages: Vec<_> = self.packages.iter().map(|e| e.value().clone()).collect();
        packages.sort_by(|a, b| b.rating.partial_cmp(&a.rating).unwrap_or(std::cmp::Ordering::Equal));
        packages.into_iter().take(limit).collect()
    }

    /// Get most downloaded packages
    pub fn get_most_downloaded(&self, limit: usize) -> Vec<ModulePackage> {
        let mut packages: Vec<_> = self.packages.iter().map(|e| e.value().clone()).collect();
        packages.sort_by(|a, b| b.downloads.cmp(&a.downloads));
        packages.into_iter().take(limit).collect()
    }

    /// Add package to category
    pub fn add_to_category(&self, category: &str, package_id: String) {
        self.categories
            .entry(category.to_string())
            .or_insert_with(Vec::new)
            .push(package_id);
    }

    /// Get packages in category
    pub fn get_category_packages(&self, category: &str) -> Vec<ModulePackage> {
        self.categories
            .get(category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.packages.get(id).map(|p| p.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Record a download
    pub fn record_download(&self, package_id: &str) -> Result<(), String> {
        self.packages
            .alter(package_id, |_, mut pkg| {
                pkg.downloads += 1;
                pkg
            });

        let mut stats = self.stats.lock();
        stats.total_downloads += 1;

        Ok(())
    }

    /// Rate a package
    pub fn rate_package(&self, package_id: &str, rating: f32) -> Result<(), String> {
        if !(0.0..=5.0).contains(&rating) {
            return Err("Rating must be between 0 and 5".to_string());
        }

        self.packages
            .alter(package_id, |_, mut pkg| {
                // Simple average (could be improved with weighted average)
                pkg.rating = (pkg.rating + rating) / 2.0;
                pkg
            });

        Ok(())
    }

    /// List all packages
    pub fn list_all_packages(&self) -> Vec<ModulePackage> {
        self.packages.iter().map(|e| e.value().clone()).collect()
    }

    /// Get marketplace statistics
    pub fn get_stats(&self) -> MarketplaceStats {
        self.stats.lock().clone()
    }
}

impl Default for ModuleMarketplace {
    fn default() -> Self {
        Self::new()
    }
}

/// Marketplace Index - fast search and discovery
pub struct MarketplaceIndex {
    marketplace: Arc<ModuleMarketplace>,
    language_index: Arc<DashMap<String, Vec<String>>>, // language -> package_ids
}

impl MarketplaceIndex {
    pub fn new(marketplace: Arc<ModuleMarketplace>) -> Self {
        MarketplaceIndex {
            marketplace,
            language_index: Arc::new(DashMap::new()),
        }
    }

    /// Update the index (should be called after marketplace changes)
    pub fn rebuild_index(&self) {
        self.language_index.clear();

        for pkg in self.marketplace.list_all_packages() {
            for lang in &pkg.languages {
                self.language_index
                    .entry(lang.clone())
                    .or_insert_with(Vec::new)
                    .push(pkg.id.clone());
            }
        }
    }

    /// Quick lookup by language
    pub fn find_by_language(&self, language: &str) -> Vec<ModulePackage> {
        self.language_index
            .get(language)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.marketplace.get_package(id))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_marketplace_publish() {
        let marketplace = ModuleMarketplace::new();

        let package = ModulePackage {
            id: "math/matrix".to_string(),
            name: "Matrix Operations".to_string(),
            description: "Linear algebra for polyglot systems".to_string(),
            languages: vec!["rust".to_string(), "python".to_string()],
            version: "1.0.0".to_string(),
            author: "Omnisystem".to_string(),
            repository: Some("https://github.com/omnisystem/matrix".to_string()),
            license: "MIT".to_string(),
            docs_url: None,
            downloads: 0,
            rating: 4.5,
            dependencies: vec![],
            published_at: "2026-06-11".to_string(),
            size_bytes: 1024 * 512,
        };

        assert!(marketplace.publish_package(package.clone()).is_ok());
        assert!(marketplace.get_package("math/matrix").is_some());
    }

    #[test]
    fn test_marketplace_search() {
        let marketplace = ModuleMarketplace::new();

        let pkg1 = ModulePackage {
            id: "crypto/aes".to_string(),
            name: "AES Encryption".to_string(),
            description: "AES encryption for all languages".to_string(),
            languages: vec!["rust".to_string(), "python".to_string(), "javascript".to_string()],
            version: "2.0.0".to_string(),
            author: "Omnisystem".to_string(),
            repository: None,
            license: "Apache-2.0".to_string(),
            docs_url: None,
            downloads: 1000,
            rating: 4.8,
            dependencies: vec![],
            published_at: "2026-06-10".to_string(),
            size_bytes: 2048,
        };

        marketplace.publish_package(pkg1).unwrap();

        // Search by language
        let results = marketplace.search_by_language("rust");
        assert_eq!(results.len(), 1);

        // Search by query
        let results = marketplace.search_by_query("encryption");
        assert_eq!(results.len(), 1);
    }
}
