//! Language Module Registry
//!
//! Tracks and manages modules written in different languages.

use crate::language::Language;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Module information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub name: String,
    pub language: Language,
    pub version: String,
    pub exported_functions: Vec<String>,
    pub dependencies: Vec<String>,
}

/// Language Module Registry
pub struct LanguageRegistry {
    modules: HashMap<String, ModuleInfo>,
}

impl LanguageRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    /// Register a module
    pub fn register_module(&mut self, name: &str, language: Language) {
        let module = ModuleInfo {
            name: name.to_string(),
            language,
            version: "1.0.0".to_string(),
            exported_functions: Vec::new(),
            dependencies: Vec::new(),
        };
        self.modules.insert(name.to_string(), module);
    }

    /// Get module info
    pub fn get_module(&self, name: &str) -> Option<ModuleInfo> {
        self.modules.get(name).cloned()
    }

    /// List all modules
    pub fn list_modules(&self) -> Vec<ModuleInfo> {
        self.modules.values().cloned().collect()
    }

    /// List modules by language
    pub fn list_by_language(&self, language: Language) -> Vec<ModuleInfo> {
        self.modules
            .values()
            .filter(|m| m.language == language)
            .cloned()
            .collect()
    }

    /// Add exported function to module
    pub fn add_export(&mut self, module: &str, function: &str) {
        if let Some(m) = self.modules.get_mut(module) {
            m.exported_functions.push(function.to_string());
        }
    }

    /// Add dependency to module
    pub fn add_dependency(&mut self, module: &str, dependency: &str) {
        if let Some(m) = self.modules.get_mut(module) {
            m.dependencies.push(dependency.to_string());
        }
    }

    /// Remove module
    pub fn remove_module(&mut self, name: &str) -> Option<ModuleInfo> {
        self.modules.remove(name)
    }
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let _registry = LanguageRegistry::new();
    }

    #[test]
    fn test_register_module() {
        let mut registry = LanguageRegistry::new();
        registry.register_module("my_module", Language::Rust);

        let module = registry.get_module("my_module");
        assert!(module.is_some());
        assert_eq!(module.unwrap().language, Language::Rust);
    }

    #[test]
    fn test_list_by_language() {
        let mut registry = LanguageRegistry::new();
        registry.register_module("rust_mod", Language::Rust);
        registry.register_module("titan_mod", Language::Titan);

        let rust_modules = registry.list_by_language(Language::Rust);
        assert_eq!(rust_modules.len(), 1);

        let titan_modules = registry.list_by_language(Language::Titan);
        assert_eq!(titan_modules.len(), 1);
    }
}
