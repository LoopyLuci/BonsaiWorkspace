/// Multi-language detection for spell checking.
/// Supports detecting text chunks in different languages within the same file.

use anyhow::Result;
use std::path::Path;

/// Detect languages in a text with per-line granularity.
pub fn detect_languages_per_line(text: &str) -> Vec<LineLanguage> {
    let mut results = Vec::new();

    for (line_no, line) in text.lines().enumerate() {
        if !line.is_empty() {
            if let Some(lang) = detect_line_language(line) {
                results.push(LineLanguage {
                    line_no: line_no as u32,
                    language: lang,
                    confidence: 0.8,
                });
            }
        }
    }

    results
}

fn detect_line_language(line: &str) -> Option<String> {
    if let Some(detected) = whatlang::detect(line) {
        match detected.lang {
            whatlang::Lang::Eng => Some("en".to_string()),
            whatlang::Lang::Spa => Some("es".to_string()),
            whatlang::Lang::Deu => Some("de".to_string()),
            whatlang::Lang::Fra => Some("fr".to_string()),
            whatlang::Lang::Por => Some("pt".to_string()),
            whatlang::Lang::Rus => Some("ru".to_string()),
            whatlang::Lang::Jpn => Some("ja".to_string()),
            whatlang::Lang::Cmn => Some("zh".to_string()),
            whatlang::Lang::Arb => Some("ar".to_string()),
            _ => None,
        }
    } else {
        None
    }
}

#[derive(Debug, Clone)]
pub struct LineLanguage {
    pub line_no: u32,
    pub language: String,
    pub confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_languages_per_line() {
        let text = "This is English.\nÉsto es español.\nDas ist Deutsch.";
        let results = detect_languages_per_line(text);
        assert!(!results.is_empty());
    }
}
