//! Language detection and support module

use crate::error::{Error, Result};
use std::path::Path;
use std::collections::HashMap;
use lazy_static::lazy_static;

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, serde::Serialize, serde::Deserialize)]
pub enum Language {
    Rust,
    C,
    Cpp,
    Titan,
    Go,
    Zig,
    Python,
    TypeScript,
    JavaScript,
    Java,
    Kotlin,
    CSharp,
    ObjectiveC,
    Swift,
    D,
    Haskell,
}

impl Language {
    /// Get language name
    pub fn name(&self) -> &'static str {
        match self {
            Language::Rust => "Rust",
            Language::C => "C",
            Language::Cpp => "C++",
            Language::Titan => "Titan",
            Language::Go => "Go",
            Language::Zig => "Zig",
            Language::Python => "Python",
            Language::TypeScript => "TypeScript",
            Language::JavaScript => "JavaScript",
            Language::Java => "Java",
            Language::Kotlin => "Kotlin",
            Language::CSharp => "C#",
            Language::ObjectiveC => "Objective-C",
            Language::Swift => "Swift",
            Language::D => "D",
            Language::Haskell => "Haskell",
        }
    }

    /// Get file extensions for this language
    pub fn file_extensions(&self) -> &'static [&'static str] {
        match self {
            Language::Rust => &["rs"],
            Language::C => &["c", "h"],
            Language::Cpp => &["cpp", "cc", "cxx", "c++", "hpp", "h++"],
            Language::Titan => &["ti"],
            Language::Go => &["go"],
            Language::Zig => &["zig"],
            Language::Python => &["py", "pyw", "pyi"],
            Language::TypeScript => &["ts", "tsx"],
            Language::JavaScript => &["js", "jsx", "mjs"],
            Language::Java => &["java"],
            Language::Kotlin => &["kt", "kts"],
            Language::CSharp => &["cs"],
            Language::ObjectiveC => &["m", "mm"],
            Language::Swift => &["swift"],
            Language::D => &["d"],
            Language::Haskell => &["hs", "lhs"],
        }
    }

    /// Check if this language is compiled (vs interpreted)
    pub fn is_compiled(&self) -> bool {
        matches!(
            self,
            Language::Rust
                | Language::C
                | Language::Cpp
                | Language::Titan
                | Language::Go
                | Language::Zig
                | Language::Java
                | Language::Kotlin
                | Language::CSharp
                | Language::ObjectiveC
                | Language::Swift
                | Language::D
                | Language::Haskell
        )
    }

    /// All supported languages
    pub fn all() -> &'static [Language] {
        &[
            Language::Rust,
            Language::C,
            Language::Cpp,
            Language::Titan,
            Language::Go,
            Language::Zig,
            Language::Python,
            Language::TypeScript,
            Language::JavaScript,
            Language::Java,
            Language::Kotlin,
            Language::CSharp,
            Language::ObjectiveC,
            Language::Swift,
            Language::D,
            Language::Haskell,
        ]
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Language confidence score (0.0 to 1.0)
#[derive(Debug, Clone, Copy)]
pub struct Confidence(pub f32);

impl Confidence {
    /// Create confidence score
    pub fn new(score: f32) -> Self {
        Confidence(score.max(0.0).min(1.0))
    }

    /// High confidence (>0.9)
    pub fn is_high(&self) -> bool {
        self.0 > 0.9
    }

    /// Medium confidence (0.7-0.9)
    pub fn is_medium(&self) -> bool {
        self.0 > 0.7 && self.0 <= 0.9
    }

    /// Low confidence (<0.7)
    pub fn is_low(&self) -> bool {
        self.0 <= 0.7
    }
}

/// Language detector
pub struct LanguageDetector {
    extension_map: HashMap<String, Language>,
}

impl LanguageDetector {
    /// Create a new language detector
    pub fn new() -> Self {
        let mut extension_map = HashMap::new();

        for lang in Language::all() {
            for ext in lang.file_extensions() {
                extension_map.insert(ext.to_lowercase(), *lang);
            }
        }

        Self { extension_map }
    }

    /// Detect language from file path
    pub fn detect(&self, path: &Path) -> Result<(Language, Confidence)> {
        // Try extension-based detection first (fastest)
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if let Some(&lang) = self.extension_map.get(&ext.to_lowercase()) {
                return Ok((lang, Confidence::new(0.95)));
            }
        }

        // Could add content-based detection here
        // For now, return error if extension doesn't match
        Err(Error::LanguageDetection {
            path: path.to_path_buf(),
            reason: "Unknown file extension".to_string(),
        })
    }

    /// Detect multiple files and return their languages
    pub fn detect_batch<'a>(&self, paths: &[&'a Path]) -> HashMap<&'a Path, Result<(Language, Confidence)>> {
        paths.iter().map(|&path| (path, self.detect(path))).collect()
    }

    /// Detect language from file content (stub for future ML implementation)
    pub fn detect_from_content(&self, content: &str) -> Option<(Language, Confidence)> {
        // Check for Rust patterns
        if content.contains("fn main()") && content.contains("use ") {
            return Some((Language::Rust, Confidence::new(0.85)));
        }

        // Check for C patterns
        if content.contains("#include") && content.contains("int main") {
            return Some((Language::C, Confidence::new(0.80)));
        }

        // Check for Go patterns
        if content.contains("package main") && content.contains("func main()") {
            return Some((Language::Go, Confidence::new(0.90)));
        }

        None
    }
}

impl Default for LanguageDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_names() {
        assert_eq!(Language::Rust.name(), "Rust");
        assert_eq!(Language::C.name(), "C");
        assert_eq!(Language::Cpp.name(), "C++");
    }

    #[test]
    fn test_file_extensions() {
        assert!(Language::Rust.file_extensions().contains(&"rs"));
        assert!(Language::C.file_extensions().contains(&"c"));
    }

    #[test]
    fn test_is_compiled() {
        assert!(Language::Rust.is_compiled());
        assert!(Language::C.is_compiled());
        assert!(Language::Python.is_compiled());
    }

    #[test]
    fn test_language_detector() {
        let detector = LanguageDetector::new();

        let path = Path::new("test.rs");
        let (lang, conf) = detector.detect(path).unwrap();
        assert_eq!(lang, Language::Rust);
        assert!(conf.is_high());
    }

    #[test]
    fn test_detect_unknown_extension() {
        let detector = LanguageDetector::new();
        let path = Path::new("test.unknown");
        assert!(detector.detect(path).is_err());
    }

    #[test]
    fn test_confidence_levels() {
        let high = Confidence::new(0.95);
        let medium = Confidence::new(0.8);
        let low = Confidence::new(0.5);

        assert!(high.is_high());
        assert!(medium.is_medium());
        assert!(low.is_low());
    }
}
