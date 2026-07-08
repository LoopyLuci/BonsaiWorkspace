//! Language Support Module
//!
//! Defines supported languages and provides initialization/management.

use crate::error::{Result, UllError};
use std::sync::Arc;
use parking_lot::RwLock;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Supported languages in Omnisystem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    /// Rust - systems programming
    Rust,
    /// TITAN - systems programming (Omni-language)
    Titan,
    /// SYLVA - machine learning (Omni-language)
    Sylva,
    /// AETHER - distributed systems (Omni-language)
    Aether,
    /// AXIOM - formal verification (Omni-language)
    Axiom,
    /// JavaScript/TypeScript - web
    JavaScript,
    /// Python - data science (legacy support)
    Python,
}

impl Language {
    /// Get language display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::Titan => "TITAN",
            Self::Sylva => "SYLVA",
            Self::Aether => "AETHER",
            Self::Axiom => "AXIOM",
            Self::JavaScript => "JavaScript",
            Self::Python => "Python",
        }
    }

    /// Check if this is an Omni-language
    pub fn is_omni_language(&self) -> bool {
        matches!(self, Self::Titan | Self::Sylva | Self::Aether | Self::Axiom)
    }

    /// Get file extension for language
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Rust => "rs",
            Self::Titan => "ti",
            Self::Sylva => "sylva",
            Self::Aether => "aether",
            Self::Axiom => "axiom",
            Self::JavaScript => "js",
            Self::Python => "py",
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Language runtime context
#[derive(Debug, Clone)]
pub struct LanguageContext {
    pub language: Language,
    pub initialized: bool,
    pub version: String,
    pub metadata: HashMap<String, String>,
}

impl LanguageContext {
    /// Create new language context
    pub fn new(language: Language, version: impl Into<String>) -> Self {
        Self {
            language,
            initialized: false,
            version: version.into(),
            metadata: HashMap::new(),
        }
    }
}

/// Global language registry
static LANGUAGE_REGISTRY: Lazy<Arc<RwLock<LanguageRegistry>>> =
    Lazy::new(|| Arc::new(RwLock::new(LanguageRegistry::new())));

pub struct LanguageRegistry {
    contexts: HashMap<Language, LanguageContext>,
}

impl LanguageRegistry {
    pub fn new() -> Self {
        Self {
            contexts: HashMap::new(),
        }
    }

    pub fn register(&mut self, context: LanguageContext) {
        self.contexts.insert(context.language, context);
    }

    pub fn get(&self, language: Language) -> Option<LanguageContext> {
        self.contexts.get(&language).cloned()
    }

    pub fn is_initialized(&self, language: Language) -> bool {
        self.contexts
            .get(&language)
            .map(|ctx| ctx.initialized)
            .unwrap_or(false)
    }

    pub fn list_languages(&self) -> Vec<LanguageContext> {
        self.contexts.values().cloned().collect()
    }
}

/// Initialize all language runtimes
pub async fn initialize_runtimes() -> Result<()> {
    let mut registry = LANGUAGE_REGISTRY.write();

    // Initialize Rust runtime (always available)
    let mut rust_ctx = LanguageContext::new(Language::Rust, "1.75");
    rust_ctx.initialized = true;
    registry.register(rust_ctx);
    log::info!("Rust runtime initialized");

    // Initialize TITAN runtime
    let mut titan_ctx = LanguageContext::new(Language::Titan, "2.0.0");
    titan_ctx.initialized = true;
    registry.register(titan_ctx);
    log::info!("TITAN runtime initialized");

    // Initialize SYLVA runtime
    let mut sylva_ctx = LanguageContext::new(Language::Sylva, "2.0.0");
    sylva_ctx.initialized = true;
    registry.register(sylva_ctx);
    log::info!("SYLVA runtime initialized");

    // Initialize AETHER runtime
    let mut aether_ctx = LanguageContext::new(Language::Aether, "2.0.0");
    aether_ctx.initialized = true;
    registry.register(aether_ctx);
    log::info!("AETHER runtime initialized");

    // Initialize AXIOM runtime
    let mut axiom_ctx = LanguageContext::new(Language::Axiom, "2.0.0");
    axiom_ctx.initialized = true;
    registry.register(axiom_ctx);
    log::info!("AXIOM runtime initialized");

    Ok(())
}

/// Shutdown all language runtimes
pub async fn shutdown_runtimes() -> Result<()> {
    let mut registry = LANGUAGE_REGISTRY.write();
    registry.contexts.clear();
    Ok(())
}

/// Get language context
pub fn get_language(language: Language) -> Result<LanguageContext> {
    let registry = LANGUAGE_REGISTRY.read();
    registry
        .get(language)
        .ok_or_else(|| UllError::LanguageNotFound(language.to_string()))
}

/// Check if language is initialized
pub fn is_language_initialized(language: Language) -> Result<bool> {
    let registry = LANGUAGE_REGISTRY.read();
    Ok(registry.is_initialized(language))
}

/// List all supported languages
pub fn list_supported_languages() -> Vec<LanguageContext> {
    let registry = LANGUAGE_REGISTRY.read();
    registry.list_languages()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_display() {
        assert_eq!(Language::Rust.display_name(), "Rust");
        assert_eq!(Language::Titan.display_name(), "TITAN");
    }

    #[test]
    fn test_language_extensions() {
        assert_eq!(Language::Rust.extension(), "rs");
        assert_eq!(Language::Titan.extension(), "ti");
        assert_eq!(Language::Sylva.extension(), "sylva");
    }

    #[test]
    fn test_omni_language() {
        assert!(Language::Titan.is_omni_language());
        assert!(!Language::Rust.is_omni_language());
    }

    #[tokio::test]
    async fn test_runtime_initialization() {
        initialize_runtimes().await.unwrap();
        let langs = list_supported_languages();
        assert!(!langs.is_empty());
        shutdown_runtimes().await.unwrap();
    }
}
