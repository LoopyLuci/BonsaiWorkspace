/// Polyglot Runtime - Multi-language execution coordination

use std::sync::Arc;
use std::collections::HashMap;
use omnisystem_ffi::FFIRegistry;
use omnisystem_loader::ModuleLoader;
use parking_lot::RwLock;
use tracing::info;

/// Language support in Omnisystem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Go,
    Python,
    JavaScript,
    Java,
    CSharp,
    Kotlin,
    Swift,
    TypeScript,
    CPlusPlus,
    Other(u32),
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Go => "go",
            Language::Python => "python",
            Language::JavaScript => "javascript",
            Language::Java => "java",
            Language::CSharp => "csharp",
            Language::Kotlin => "kotlin",
            Language::Swift => "swift",
            Language::TypeScript => "typescript",
            Language::CPlusPlus => "cpp",
            Language::Other(_) => "other",
        }
    }

    pub fn version(&self) -> (u32, u32, u32) {
        match self {
            Language::Rust => (1, 0, 0),
            Language::Go => (1, 0, 0),
            Language::Python => (1, 0, 0),
            Language::JavaScript => (1, 0, 0),
            Language::Java => (1, 0, 0),
            Language::CSharp => (1, 0, 0),
            Language::Kotlin => (1, 0, 0),
            Language::Swift => (1, 0, 0),
            Language::TypeScript => (1, 0, 0),
            Language::CPlusPlus => (1, 0, 0),
            Language::Other(_) => (0, 0, 0),
        }
    }
}

/// Language module metadata
#[derive(Debug, Clone, Copy)]
pub struct LanguageModule {
    pub language: Language,
    pub version: (u32, u32, u32),
    pub loaded: bool,
}

/// Polyglot runtime for multi-language execution
pub struct PolyglotRuntime {
    ffi_registry: Arc<FFIRegistry>,
    #[allow(dead_code)]
    module_loader: Arc<ModuleLoader>,
    loaded_languages: RwLock<HashMap<String, LanguageModule>>,
}

impl PolyglotRuntime {
    pub fn new(
        ffi_registry: Arc<FFIRegistry>,
        module_loader: Arc<ModuleLoader>,
    ) -> Self {
        info!("Initializing Polyglot Runtime for 750+ languages");

        PolyglotRuntime {
            ffi_registry,
            module_loader,
            loaded_languages: RwLock::new(HashMap::new()),
        }
    }

    /// Register a language binding
    pub async fn register_language(&self, language: Language) -> Result<(), String> {
        info!("Registering language: {:?}", language);

        let (version_major, version_minor, version_patch) = language.version();

        // Create FFI module for the language
        let module = Arc::new(omnisystem_ffi::FFIModule::new(
            language.as_str(),
            (version_major, version_minor, version_patch),
        ));

        self.ffi_registry.register_module(module);

        // Record in loaded languages
        let mut loaded = self.loaded_languages.write();
        loaded.insert(
            language.as_str().to_string(),
            LanguageModule {
                language,
                version: (version_major, version_minor, version_patch),
                loaded: true,
            },
        );

        info!("Language registered: {} v{}.{}.{}", language.as_str(), version_major, version_minor, version_patch);

        Ok(())
    }

    /// Get list of supported languages
    pub fn supported_languages() -> Vec<Language> {
        vec![
            Language::Rust,
            Language::Go,
            Language::Python,
            Language::JavaScript,
            Language::Java,
            Language::CSharp,
            Language::Kotlin,
            Language::Swift,
            Language::TypeScript,
            Language::CPlusPlus,
        ]
    }

    /// List loaded languages
    pub fn list_loaded_languages(&self) -> Vec<String> {
        self.loaded_languages
            .read()
            .keys()
            .cloned()
            .collect()
    }

    /// Get number of loaded languages
    pub fn loaded_language_count(&self) -> usize {
        self.loaded_languages.read().len()
    }

    /// Check if language is loaded
    pub fn is_language_loaded(&self, language: Language) -> bool {
        self.loaded_languages
            .read()
            .contains_key(language.as_str())
    }

    /// Get language module info
    pub fn get_language_info(&self, language: Language) -> Option<LanguageModule> {
        self.loaded_languages
            .read()
            .get(language.as_str())
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_as_str() {
        assert_eq!(Language::Rust.as_str(), "rust");
        assert_eq!(Language::Go.as_str(), "go");
        assert_eq!(Language::Python.as_str(), "python");
    }

    #[test]
    fn test_language_version() {
        let v = Language::Rust.version();
        assert_eq!(v, (1, 0, 0));
    }

    #[test]
    fn test_supported_languages() {
        let langs = PolyglotRuntime::supported_languages();
        assert!(langs.len() >= 10);
        assert!(langs.contains(&Language::Rust));
        assert!(langs.contains(&Language::Python));
    }
}
