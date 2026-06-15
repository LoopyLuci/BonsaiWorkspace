//! Omnisystem Module Marketplace
//!
//! Distributed module registry and discovery system with:
//! - Module discovery and search
//! - Installation management
//! - Version management
//! - Rating and review system
//! - Dependency resolution

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Module listing in the marketplace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleListing {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub documentation: Option<String>,
    pub license: String,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
    pub downloads: u64,
    pub rating: f32,
    pub review_count: u32,
    pub published_at: String,
    pub last_updated: String,
    pub capabilities: Vec<String>,
    pub dependencies: Vec<ModuleDependency>,
}

/// Module dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDependency {
    pub name: String,
    pub version_requirement: String,
}

/// Module installation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInstallation {
    pub id: String,
    pub name: String,
    pub version: String,
    pub installed_at: String,
    pub enabled: bool,
    pub auto_update: bool,
    pub config: serde_json::Value,
}

/// Marketplace search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub query: String,
    pub results: Vec<ModuleListing>,
    pub total_count: u32,
    pub page: u32,
    pub per_page: u32,
}

/// Module rating and review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleReview {
    pub id: String,
    pub module_name: String,
    pub rating: u8,
    pub title: String,
    pub content: String,
    pub author: String,
    pub created_at: String,
    pub helpful_count: u32,
}

/// Marketplace registry service
pub struct Marketplace {
    modules: HashMap<String, ModuleListing>,
    installations: HashMap<String, ModuleInstallation>,
    reviews: HashMap<String, Vec<ModuleReview>>,
}

impl Marketplace {
    /// Create new marketplace
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            installations: HashMap::new(),
            reviews: HashMap::new(),
        }
    }

    /// Search for modules
    pub fn search(&self, query: &str, page: u32, per_page: u32) -> SearchResult {
        let results: Vec<ModuleListing> = self
            .modules
            .values()
            .filter(|m| {
                m.name.to_lowercase().contains(&query.to_lowercase())
                    || m.description.to_lowercase().contains(&query.to_lowercase())
                    || m.keywords.iter().any(|k| k.to_lowercase().contains(&query.to_lowercase()))
            })
            .skip((page as usize - 1) * per_page as usize)
            .take(per_page as usize)
            .cloned()
            .collect();

        SearchResult {
            query: query.to_string(),
            total_count: self.modules.len() as u32,
            results,
            page,
            per_page,
        }
    }

    /// Get module by name
    pub fn get_module(&self, name: &str) -> Option<ModuleListing> {
        self.modules.get(name).cloned()
    }

    /// List all modules
    pub fn list_modules(&self) -> Vec<ModuleListing> {
        self.modules.values().cloned().collect()
    }

    /// Install a module
    pub fn install_module(
        &mut self,
        name: &str,
        version: &str,
    ) -> Result<ModuleInstallation, String> {
        let module = self
            .modules
            .get(name)
            .ok_or_else(|| format!("Module {} not found", name))?;

        let installation = ModuleInstallation {
            id: uuid::Uuid::new_v4().to_string(),
            name: module.name.clone(),
            version: version.to_string(),
            installed_at: chrono::Utc::now().to_rfc3339(),
            enabled: true,
            auto_update: true,
            config: serde_json::json!({}),
        };

        self.installations
            .insert(installation.id.clone(), installation.clone());
        Ok(installation)
    }

    /// Uninstall a module
    pub fn uninstall_module(&mut self, name: &str) -> Result<(), String> {
        self.installations
            .retain(|_, v| v.name != name);
        Ok(())
    }

    /// Get module reviews
    pub fn get_reviews(&self, module_name: &str) -> Vec<ModuleReview> {
        self.reviews
            .get(module_name)
            .map(|reviews| reviews.clone())
            .unwrap_or_default()
    }

    /// Add review
    pub fn add_review(
        &mut self,
        module_name: &str,
        rating: u8,
        title: &str,
        content: &str,
        author: &str,
    ) -> ModuleReview {
        let review = ModuleReview {
            id: uuid::Uuid::new_v4().to_string(),
            module_name: module_name.to_string(),
            rating,
            title: title.to_string(),
            content: content.to_string(),
            author: author.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            helpful_count: 0,
        };

        self.reviews
            .entry(module_name.to_string())
            .or_insert_with(Vec::new)
            .push(review.clone());

        review
    }
}

impl Default for Marketplace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marketplace_creation() {
        let marketplace = Marketplace::new();
        assert_eq!(marketplace.modules.len(), 0);
    }

    #[test]
    fn test_marketplace_search() {
        let marketplace = Marketplace::new();
        let result = marketplace.search("compiler", 1, 10);
        assert_eq!(result.query, "compiler");
    }
}
