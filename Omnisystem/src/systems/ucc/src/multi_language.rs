//! Multi-Language Builder - Orchestrates compilation of polyglot projects
//!
//! Handles detection, compilation, and linking of projects containing
//! multiple programming languages.

use crate::error::Result;
use crate::language::{Language, LanguageDetector};
use crate::compiler_registry::CompilerRegistry;
use crate::core::{CompileTarget, BuildStats};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

/// Multi-language build result
#[derive(Debug, Clone)]
pub struct MultiLanguageBuildResult {
    /// Language -> build success
    pub language_results: HashMap<Language, bool>,
    /// Total time in milliseconds
    pub total_time_ms: u128,
    /// Languages compiled
    pub compiled_languages: Vec<Language>,
    /// Overall success
    pub success: bool,
}

/// Orchestrates compilation of multi-language projects
pub struct MultiLanguageBuilder {
    registry: CompilerRegistry,
    detector: LanguageDetector,
    project_root: PathBuf,
    target: CompileTarget,
}

impl MultiLanguageBuilder {
    /// Create a new multi-language builder
    pub fn new(
        registry: CompilerRegistry,
        project_root: PathBuf,
        target: CompileTarget,
    ) -> Self {
        Self {
            registry,
            detector: LanguageDetector::new(),
            project_root,
            target,
        }
    }

    /// Detect all languages in the project
    pub fn detect_languages(&self) -> Result<Vec<Language>> {
        let mut languages = std::collections::HashSet::new();

        for entry in walkdir::WalkDir::new(&self.project_root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            if let Ok((lang, _conf)) = self.detector.detect(entry.path()) {
                languages.insert(lang);
            }
        }

        let mut result: Vec<_> = languages.into_iter().collect();
        result.sort();
        Ok(result)
    }

    /// Get compilation order (dependency-aware if possible)
    fn get_compilation_order(&self, languages: &[Language]) -> VecDeque<Language> {
        // Simple ordering: compile Rust first (has linker), then others
        let mut ordered = VecDeque::new();

        for lang in languages {
            match lang {
                Language::Rust => ordered.push_front(*lang),
                _ => ordered.push_back(*lang),
            }
        }

        ordered
    }

    /// Build all languages in the project
    pub async fn build(&self) -> Result<MultiLanguageBuildResult> {
        let start = std::time::Instant::now();

        // Detect languages
        let languages = self.detect_languages()?;
        let mut result = MultiLanguageBuildResult {
            language_results: HashMap::new(),
            total_time_ms: 0,
            compiled_languages: Vec::new(),
            success: true,
        };

        if languages.is_empty() {
            return Ok(result);
        }

        // Get compilation order
        let order = self.get_compilation_order(&languages);

        // Compile each language
        for lang in order {
            match self.registry.compile(lang, &[], &self.target).await {
                Ok(compile_result) => {
                    result.language_results.insert(lang, compile_result.success);
                    result.compiled_languages.push(lang);
                    if !compile_result.success {
                        result.success = false;
                    }
                }
                Err(_e) => {
                    result.language_results.insert(lang, false);
                    result.success = false;
                }
            }
        }

        result.total_time_ms = start.elapsed().as_millis();
        Ok(result)
    }

    /// Check if a language is supported
    pub fn supports(&self, language: Language) -> bool {
        self.registry.supports(language)
    }

    /// Get all supported languages in registry
    pub fn supported_languages(&self) -> Vec<Language> {
        self.registry.supported_languages()
    }
}

impl MultiLanguageBuildResult {
    /// Get count of successful language compilations
    pub fn successful_languages(&self) -> usize {
        self.language_results.values().filter(|&&success| success).count()
    }

    /// Get count of failed language compilations
    pub fn failed_languages(&self) -> usize {
        self.language_results.values().filter(|&&success| !success).count()
    }

    /// Convert to BuildStats
    pub fn to_build_stats(&self) -> BuildStats {
        let mut stats = BuildStats::new();
        stats.total_units = self.compiled_languages.len();
        stats.compiled_units = self.successful_languages();
        stats.failed_units = self.failed_languages();
        stats.total_duration_ms = self.total_time_ms;
        stats.output = format!(
            "✅ Compiled {} languages in {}ms",
            self.successful_languages(),
            self.total_time_ms
        );
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creation() {
        let registry = CompilerRegistry::default();
        let builder = MultiLanguageBuilder::new(registry, PathBuf::from("."), CompileTarget::native());
        assert!(!builder.supported_languages().is_empty() == false);
    }

    #[test]
    fn test_build_result_stats() {
        let mut result = MultiLanguageBuildResult {
            language_results: HashMap::new(),
            total_time_ms: 1000,
            compiled_languages: vec![Language::Rust, Language::Go],
            success: true,
        };
        result.language_results.insert(Language::Rust, true);
        result.language_results.insert(Language::Go, true);

        assert_eq!(result.successful_languages(), 2);
        assert_eq!(result.failed_languages(), 0);
    }
}
