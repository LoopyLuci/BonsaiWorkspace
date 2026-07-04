//! Compiler Registry - Manages all language-specific compilers
//!
//! Concrete enum-based registry for all supported languages.
//! Avoids dyn trait issues with async methods.

use crate::error::Result;
use crate::language::Language;
use crate::compiler::LanguageCompiler;
use crate::compilers::{CppCompiler, GoCompiler, ZigCompiler};
use crate::compiler::RustCompiler;
use crate::core::{CompileTarget, CompileResult};
use std::path::{Path, PathBuf};
use std::fmt;

/// Concrete enum of all supported compilers
pub enum ConcreteCompiler {
    Rust(RustCompiler),
    Cpp(CppCompiler),
    Go(GoCompiler),
    Zig(ZigCompiler),
}

impl ConcreteCompiler {
    /// Compile with the appropriate compiler
    pub async fn compile(
        &self,
        sources: &[&Path],
        target: &CompileTarget,
    ) -> Result<CompileResult> {
        match self {
            ConcreteCompiler::Rust(c) => c.compile(sources, target).await,
            ConcreteCompiler::Cpp(c) => c.compile(sources, target).await,
            ConcreteCompiler::Go(c) => c.compile(sources, target).await,
            ConcreteCompiler::Zig(c) => c.compile(sources, target).await,
        }
    }

    /// Check availability
    pub fn check_availability(&self) -> Result<()> {
        match self {
            ConcreteCompiler::Rust(c) => c.check_availability(),
            ConcreteCompiler::Cpp(c) => c.check_availability(),
            ConcreteCompiler::Go(c) => c.check_availability(),
            ConcreteCompiler::Zig(c) => c.check_availability(),
        }
    }

    /// Get version
    pub fn get_version(&self) -> Result<String> {
        match self {
            ConcreteCompiler::Rust(c) => c.get_version(),
            ConcreteCompiler::Cpp(c) => c.get_version(),
            ConcreteCompiler::Go(c) => c.get_version(),
            ConcreteCompiler::Zig(c) => c.get_version(),
        }
    }
}

/// Registry for all available language compilers
pub struct CompilerRegistry {
    compilers: std::collections::HashMap<Language, ConcreteCompiler>,
}

impl CompilerRegistry {
    /// Create a new, empty compiler registry
    pub fn new() -> Self {
        Self {
            compilers: std::collections::HashMap::new(),
        }
    }

    /// Register a compiler for a language
    pub fn register(&mut self, language: Language, compiler: ConcreteCompiler) -> Result<()> {
        self.compilers.insert(language, compiler);
        Ok(())
    }

    /// Get a compiler for a language
    pub fn get(&self, language: Language) -> Option<&ConcreteCompiler> {
        self.compilers.get(&language)
    }

    /// Check if a language is supported
    pub fn supports(&self, language: Language) -> bool {
        self.compilers.contains_key(&language)
    }

    /// Get all supported languages
    pub fn supported_languages(&self) -> Vec<Language> {
        self.compilers.keys().copied().collect()
    }

    /// Get number of registered compilers
    pub fn count(&self) -> usize {
        self.compilers.len()
    }

    /// Compile sources with the appropriate language compiler
    pub async fn compile(
        &self,
        language: Language,
        sources: &[&Path],
        target: &CompileTarget,
    ) -> Result<CompileResult> {
        let compiler = self
            .get(language)
            .ok_or_else(|| crate::error::Error::CompilerNotFound {
                compiler: format!("{:?}", language),
                language: language.name().to_string(),
            })?;

        compiler.compile(sources, target).await
    }

    /// Clear all registered compilers
    pub fn clear(&mut self) {
        self.compilers.clear();
    }
}

impl Default for CompilerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for CompilerRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let langs: Vec<_> = self.supported_languages();
        f.debug_struct("CompilerRegistry")
            .field("supported_languages", &langs)
            .field("count", &self.count())
            .finish()
    }
}

impl Clone for CompilerRegistry {
    fn clone(&self) -> Self {
        // Since ConcreteCompiler doesn't impl Clone, we create a new empty registry
        // This is a limitation of the current approach
        let mut registry = Self::new();
        // In a real scenario, we'd need to re-register compilers
        // For now, this returns an empty registry
        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = CompilerRegistry::new();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_supports() {
        let registry = CompilerRegistry::new();
        let langs = registry.supported_languages();
        assert!(langs.is_empty());
    }
}
