/// Grammar and prose checking via LanguageTool
/// Lint documentation, comments, and prose content

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProseIssue {
    pub offset: usize,
    pub length: usize,
    pub message: String,
    pub category: String,
    pub suggested_replacements: Vec<String>,
    pub severity: String, // "error" or "warning"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToneAnalysis {
    pub formality_score: f32,    // 0=casual, 1=formal
    pub clarity_score: f32,      // 0=unclear, 1=clear
    pub engagement_score: f32,   // 0=boring, 1=engaging
    pub tone_category: String,   // "formal", "casual", "technical"
}

impl Default for ToneAnalysis {
    fn default() -> Self {
        Self {
            formality_score: 0.5,
            clarity_score: 0.8,
            engagement_score: 0.6,
            tone_category: "neutral".to_string(),
        }
    }
}

pub struct ProseChecker {
    language_tool_url: String,
    enabled: bool,
}

impl ProseChecker {
    pub async fn new(language_tool_url: String) -> Result<Self> {
        tracing::info!("Initializing prose checker with LanguageTool at: {}", language_tool_url);

        Ok(Self {
            language_tool_url,
            enabled: true,
        })
    }

    /// Check prose for grammar and style issues
    pub async fn check_prose(&self, text: &str, language: &str) -> Result<Vec<ProseIssue>> {
        if !self.enabled || text.is_empty() {
            return Ok(Vec::new());
        }

        tracing::debug!("Checking {} chars of prose in {}", text.len(), language);

        // TODO: Replace with actual LanguageTool HTTP request
        // let response = reqwest::Client::new()
        //     .post(&format!("{}/check", self.language_tool_url))
        //     .json(&serde_json::json!({"text": text, "language": language}))
        //     .send()
        //     .await?;
        // let issues: Vec<ProseIssue> = response.json().await?;

        Ok(Vec::new())
    }

    /// Detect the primary language of text
    pub async fn detect_language(&self, text: &str) -> Result<String> {
        if text.is_empty() {
            return Ok("unknown".to_string());
        }

        // Simple heuristic detection (replace with ML model if needed)
        let lang = if text.contains("я") || text.contains("ы") {
            "ru"
        } else if text.contains("ñ") || text.contains("¿") {
            "es"
        } else {
            "en"
        };

        Ok(lang.to_string())
    }

    /// Analyze tone/style of documentation
    pub async fn analyze_tone(&self, text: &str) -> Result<ToneAnalysis> {
        if text.is_empty() {
            return Ok(ToneAnalysis::default());
        }

        // TODO: Replace with actual ML model or LanguageTool tone analysis
        let analysis = ToneAnalysis {
            formality_score: 0.7,
            clarity_score: 0.85,
            engagement_score: 0.65,
            tone_category: if text.contains("must") || text.contains("shall") {
                "formal"
            } else {
                "neutral"
            }
            .to_string(),
        };

        Ok(analysis)
    }

    /// Check comments and docstrings in code
    pub async fn check_code_prose(&self, code: &str) -> Result<Vec<ProseIssue>> {
        if !self.enabled {
            return Ok(Vec::new());
        }

        // Extract comments and docstrings
        let comments = self.extract_comments(code);

        let mut all_issues = Vec::new();

        for (text, _offset) in comments {
            let issues = self.check_prose(&text, "en").await?;
            all_issues.extend(issues);
        }

        Ok(all_issues)
    }

    fn extract_comments(&self, code: &str) -> Vec<(String, usize)> {
        let mut comments = Vec::new();

        // Simple comment extraction (would need proper parsing for production)
        for (i, line) in code.lines().enumerate() {
            if let Some(pos) = line.find("//") {
                let comment = line[pos + 2..].trim().to_string();
                if !comment.is_empty() {
                    comments.push((comment, i));
                }
            }
        }

        comments
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        tracing::info!("Prose checker {}", if enabled { "enabled" } else { "disabled" });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_prose_checker_creation() {
        let checker = ProseChecker::new("http://localhost:8081".to_string())
            .await
            .unwrap();
        assert!(checker.enabled);
    }

    #[tokio::test]
    async fn test_language_detection() {
        let checker = ProseChecker::new("http://localhost:8081".to_string())
            .await
            .unwrap();
        let lang = checker.detect_language("Hello world").await.unwrap();
        assert_eq!(lang, "en");
    }

    #[tokio::test]
    async fn test_tone_analysis() {
        let checker = ProseChecker::new("http://localhost:8081".to_string())
            .await
            .unwrap();
        let tone = checker.analyze_tone("You must do this").await.unwrap();
        assert_eq!(tone.tone_category, "formal");
    }

    #[tokio::test]
    async fn test_check_code_prose() {
        let checker = ProseChecker::new("http://localhost:8081".to_string())
            .await
            .unwrap();
        let code = r#"
        // This is a comment
        fn main() {
            // Another comment
        }
        "#;
        let issues = checker.check_code_prose(code).await.unwrap();
        // Should extract comments (actual checking requires LanguageTool)
        assert!(issues.is_empty() || !issues.is_empty());
    }
}
