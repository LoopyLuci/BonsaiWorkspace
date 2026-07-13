pub mod hunspell_lsp;
pub mod hunspell_server;
pub mod lang_detect;
pub mod code_text_split;

use crate::diagnostics::{Diagnostic, Range, Position, Severity};
use anyhow::Result;
use std::path::Path;
use whatlang::Lang;

pub use hunspell_server::HunspellLspServer;

/// Spell checker configuration.
pub struct SpellChecker {
    /// Hunspell dictionaries for different languages
    language_map: std::collections::HashMap<String, String>,
    detect_language: bool,
}

impl SpellChecker {
    pub fn new() -> Self {
        let mut language_map = std::collections::HashMap::new();
        language_map.insert("en".to_string(), "en_US".to_string());
        language_map.insert("es".to_string(), "es_ES".to_string());
        language_map.insert("de".to_string(), "de_DE".to_string());
        language_map.insert("fr".to_string(), "fr_FR".to_string());

        Self {
            language_map,
            detect_language: true,
        }
    }

    /// Check text for spelling and grammar issues.
    pub async fn check(&self, text: &str, file: &Path, language: Option<&str>) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = Vec::new();

        if let Some(lang) = language {
            diagnostics.extend(self.check_with_language(text, file, lang).await?);
        } else if self.detect_language {
            // Auto-detect language
            if let Some(detected) = self.detect_language_of_text(text) {
                diagnostics.extend(self.check_with_language(text, file, &detected).await?);
            }
        }

        Ok(diagnostics)
    }

    async fn check_with_language(&self, text: &str, file: &Path, language: &str) -> Result<Vec<Diagnostic>> {
        // TODO: Integrate with Hunspell LSP server
        // For now, return empty
        Ok(Vec::new())
    }

    fn detect_language_of_text(&self, text: &str) -> Option<String> {
        if let Some(lang) = whatlang::detect(text) {
            match lang.lang {
                Lang::Eng => Some("en".to_string()),
                Lang::Spa => Some("es".to_string()),
                Lang::Deu => Some("de".to_string()),
                Lang::Fra => Some("fr".to_string()),
                Lang::Por => Some("pt".to_string()),
                Lang::Rus => Some("ru".to_string()),
                Lang::Jpn => Some("ja".to_string()),
                Lang::Cmn => Some("zh".to_string()),
                Lang::Arb => Some("ar".to_string()),
                _ => None,
            }
        } else {
            None
        }
    }
}

pub struct LanguageDetector;

impl LanguageDetector {
    /// Detect the human language(s) in a text.
    pub fn detect(text: &str) -> Vec<DetectedLanguage> {
        let mut results = Vec::new();

        // Use whatlang for basic detection
        if let Some(lang_info) = whatlang::detect(text) {
            results.push(DetectedLanguage {
                language: format!("{:?}", lang_info.lang).to_lowercase(),
                confidence: lang_info.confidence as f32,
            });
        }

        results
    }
}

#[derive(Debug, Clone)]
pub struct DetectedLanguage {
    pub language: String,
    pub confidence: f32,
}

impl Default for SpellChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        let english_text = "This is a test of the spell checker.";
        let detected = LanguageDetector::detect(english_text);
        assert!(!detected.is_empty());

        let spanish_text = "Esta es una prueba del corrector ortográfico.";
        let detected_es = LanguageDetector::detect(spanish_text);
        assert!(!detected_es.is_empty());
    }

    #[tokio::test]
    async fn test_spell_checker() -> Result<()> {
        let checker = SpellChecker::new();
        let text = "This is a test";
        let diags = checker.check(text, Path::new("test.txt"), Some("en")).await?;
        // Should not have errors in correct text
        assert!(diags.is_empty());
        Ok(())
    }
}
