//! Omnisystem Universal Module and Package Manager
//!
//! Language-agnostic module and package management system that:
//! - Loads modules from any programming language
//! - Manages dependencies intelligently
//! - Works with multiple package registries
//! - Provides fast, robust, simple API
//! - Handles versioning and compatibility

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

pub mod adapters;
pub mod adapters_extended;
pub mod manager;
pub mod registry;
pub mod resolver;

pub use adapters::{LanguageAdapter, LanguageAdapterTrait};
pub use adapters_extended::{ClojureAdapter, PhpAdapter, RAdapter, RubyAdapter, ScalaAdapter};
pub use manager::{ModuleManager, ModuleManagerStats};
pub use registry::{InMemoryRegistry, PackageMetadata, PackageRegistry, RegistrySearchResult};
pub use resolver::{DependencyResolver, ResolutionResult};

/// Universal module identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleId {
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub language: String,
}

impl ModuleId {
    pub fn new(namespace: &str, name: &str, version: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
            name: name.to_string(),
            version: version.to_string(),
            language: "rust".to_string(),
        }
    }

    pub fn with_language(namespace: &str, name: &str, version: &str, language: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
            name: name.to_string(),
            version: version.to_string(),
            language: language.to_string(),
        }
    }

    pub fn full_id(&self) -> String {
        format!("{}:{}@{}", self.namespace, self.name, self.version)
    }
}

/// Module metadata (language-agnostic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetadata {
    pub id: ModuleId,
    pub language: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub dependencies: Vec<ModuleDependency>,
    pub entry_point: Option<String>,
    pub exports: Vec<String>,
    pub capabilities: Vec<String>,
    pub checksum: String,
}

/// Module dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDependency {
    pub id: ModuleId,
    pub version_requirement: String,
    pub optional: bool,
}

/// Loading result
#[derive(Debug)]
pub struct LoadedModule {
    pub metadata: ModuleMetadata,
    pub location: PathBuf,
    pub loaded_at: chrono::DateTime<chrono::Utc>,
}

/// Error types
#[derive(Error, Debug)]
pub enum ModuleManagerError {
    #[error("Module not found: {0}")]
    ModuleNotFound(String),

    #[error("Dependency resolution failed: {0}")]
    DependencyResolutionFailed(String),

    #[error("Language adapter not found: {0}")]
    LanguageAdapterNotFound(String),

    #[error("Module loading failed: {0}")]
    LoadingFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Version mismatch: {0}")]
    VersionMismatch(String),

    #[error("Invalid module: {0}")]
    InvalidModule(String),

    #[error("Network error: {0}")]
    NetworkError(String),
}

pub type Result<T> = std::result::Result<T, ModuleManagerError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_id_creation() {
        let id = ModuleId::new("omnisystem", "compiler", "1.0.0");
        assert_eq!(id.full_id(), "omnisystem:compiler@1.0.0");
    }
}
