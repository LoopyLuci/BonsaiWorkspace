//! Intent Classification System
//!
//! Supports HDE embedding-based classification with deterministic fallback.
//! Maintains a known pattern database for high-confidence classification.

use serde::{Deserialize, Serialize};

/// Result of intent classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentResult {
    /// Identified intent (e.g., "start_service", "create_environment")
    pub intent: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
    /// Diagnostic notes
    pub notes: Vec<String>,
}

/// Known pattern for intent matching
#[derive(Debug, Clone)]
struct KnownPattern {
    /// Keywords that should trigger this pattern
    keywords: Vec<String>,
    /// Intent to assign
    intent: String,
    /// Confidence boost for keyword match
    confidence_boost: f64,
}

impl KnownPattern {
    fn new(keywords: Vec<&str>, intent: &str, boost: f64) -> Self {
        Self {
            keywords: keywords.into_iter().map(|s| s.to_lowercase()).collect(),
            intent: intent.to_string(),
            confidence_boost: boost,
        }
    }
}

/// Intent classifier with pattern matching and optional HDE support
pub struct IntentClassifier {
    /// Known patterns for deterministic matching
    patterns: Vec<KnownPattern>,
}

impl IntentClassifier {
    /// Create a new intent classifier with default patterns
    pub fn new() -> Self {
        let patterns = vec![
            // Service commands
            KnownPattern::new(
                vec!["start", "launch", "begin", "activate"],
                "start_service",
                0.15,
            ),
            KnownPattern::new(
                vec!["stop", "terminate", "end", "kill", "shutdown"],
                "stop_service",
                0.15,
            ),
            KnownPattern::new(
                vec!["restart", "reload", "reboot"],
                "restart_service",
                0.15,
            ),
            KnownPattern::new(
                vec!["status", "info", "check", "health"],
                "get_service_status",
                0.15,
            ),
            KnownPattern::new(
                vec!["configure", "config", "set", "update"],
                "configure_service",
                0.12,
            ),

            // Environment commands
            KnownPattern::new(
                vec!["create", "provision", "new", "setup", "spawn"],
                "create_environment",
                0.18,
            ),
            KnownPattern::new(
                vec!["snapshot", "snap", "save", "backup"],
                "snapshot_environment",
                0.15,
            ),
            KnownPattern::new(
                vec!["restore", "recover", "revert", "rollback"],
                "restore_environment",
                0.15,
            ),
            KnownPattern::new(
                vec!["migrate", "move", "transfer"],
                "migrate_environment",
                0.12,
            ),
            KnownPattern::new(
                vec!["start", "launch", "begin"],
                "start_environment",
                0.12,
            ),
            KnownPattern::new(
                vec!["stop", "halt", "pause"],
                "stop_environment",
                0.12,
            ),
            KnownPattern::new(
                vec!["delete", "remove", "destroy", "purge"],
                "delete_environment",
                0.15,
            ),

            // Module commands
            KnownPattern::new(
                vec!["install", "add", "deploy"],
                "install_module",
                0.15,
            ),
            KnownPattern::new(
                vec!["update", "upgrade", "patch"],
                "update_module",
                0.15,
            ),
            KnownPattern::new(
                vec!["remove", "uninstall", "delete"],
                "remove_module",
                0.12,
            ),

            // Asset commands
            KnownPattern::new(
                vec!["generate", "create", "make", "build"],
                "generate_asset",
                0.12,
            ),
            KnownPattern::new(
                vec!["publish", "release", "push"],
                "publish_asset",
                0.15,
            ),

            // Validation commands
            KnownPattern::new(
                vec!["validate", "test", "check", "verify", "run"],
                "run_validation",
                0.12,
            ),

            // HDE commands
            KnownPattern::new(
                vec!["toggle", "enable", "disable", "ai"],
                "toggle_ai_advisor",
                0.15,
            ),

            // Driver conversion
            KnownPattern::new(
                vec!["convert", "transform", "translate"],
                "convert_driver",
                0.12,
            ),
        ];

        Self { patterns }
    }

    /// Classify the intent of a command
    pub fn classify(&self, input: &str) -> Result<IntentResult, crate::ParseError> {
        let lower = input.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();

        if words.is_empty() {
            return Err(crate::ParseError::EmptyInput);
        }

        // Try exact word matches first
        let mut best_match: Option<(String, f64)> = None;
        let mut matched_patterns = Vec::new();

        for pattern in &self.patterns {
            for keyword in &pattern.keywords {
                // Check exact word matches
                if words.iter().any(|w| w == keyword) {
                    let confidence = 0.6 + pattern.confidence_boost;
                    matched_patterns.push((pattern.intent.clone(), confidence));

                    if best_match.is_none()
                        || confidence > best_match.as_ref().unwrap().1
                    {
                        best_match = Some((pattern.intent.clone(), confidence));
                    }
                    break;
                }
            }
        }

        // Try substring/contains matching if no exact match
        if best_match.is_none() {
            for pattern in &self.patterns {
                for keyword in &pattern.keywords {
                    if lower.contains(keyword) {
                        let confidence = 0.45 + (pattern.confidence_boost * 0.8);
                        if best_match.is_none()
                            || confidence > best_match.as_ref().unwrap().1
                        {
                            best_match = Some((pattern.intent.clone(), confidence));
                        }
                        break;
                    }
                }
            }
        }

        match best_match {
            Some((intent, confidence)) => Ok(IntentResult {
                intent,
                confidence: confidence.min(0.99),
                notes: matched_patterns
                    .into_iter()
                    .map(|(i, c)| format!("Matched: {} (confidence: {:.2})", i, c))
                    .collect(),
            }),
            None => Err(crate::ParseError::UnrecognizedCommand {
                input: input.to_string(),
            }),
        }
    }

    /// Add a custom pattern for a specific domain
    pub fn add_pattern(&mut self, keywords: Vec<&str>, intent: &str, boost: f64) {
        self.patterns
            .push(KnownPattern::new(keywords, intent, boost));
    }
}

impl Default for IntentClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_start_classification() {
        let classifier = IntentClassifier::new();
        let result = classifier.classify("start the web service").unwrap();
        assert_eq!(result.intent, "start_service");
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_environment_create_classification() {
        let classifier = IntentClassifier::new();
        let result = classifier.classify("create a new environment").unwrap();
        assert_eq!(result.intent, "create_environment");
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_module_install_classification() {
        let classifier = IntentClassifier::new();
        let result = classifier.classify("install the database module").unwrap();
        assert_eq!(result.intent, "install_module");
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_unrecognized_intent() {
        let classifier = IntentClassifier::new();
        let result = classifier.classify("xyzzy foobar baz");
        assert!(result.is_err());
    }

    #[test]
    fn test_custom_pattern() {
        let mut classifier = IntentClassifier::new();
        classifier.add_pattern(vec!["foobar"], "custom_intent", 0.2);
        let result = classifier.classify("execute foobar command").unwrap();
        assert_eq!(result.intent, "custom_intent");
    }

    #[test]
    fn test_confidence_scores() {
        let classifier = IntentClassifier::new();
        let result = classifier.classify("start service").unwrap();
        assert!(result.confidence >= 0.65 && result.confidence <= 0.99);
    }
}
