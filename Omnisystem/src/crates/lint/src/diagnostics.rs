use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A single linting diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    /// Unique identifier for the rule that generated this diagnostic.
    pub rule_id: String,

    /// Human-readable message.
    pub message: String,

    /// Severity level.
    pub severity: Severity,

    /// File path where the issue was found.
    pub file: PathBuf,

    /// Range in the file (start line:col, end line:col).
    pub range: Range,

    /// Optional fix suggestion.
    pub fix: Option<Fix>,

    /// Confidence score (0.0–1.0). Used by AI post-processing to filter false positives.
    pub confidence: f32,

    /// Optional explanation of why this is an issue.
    pub explanation: Option<String>,
}

/// Line and column numbers (0-indexed).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

/// Byte-based range in the source file.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Range {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position, start_byte: usize, end_byte: usize) -> Self {
        Self { start, end, start_byte, end_byte }
    }
}

/// Severity level for a diagnostic.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Severity {
    #[serde(rename = "note")]
    Note = 0,
    #[serde(rename = "hint")]
    Hint = 1,
    #[serde(rename = "warning")]
    Warning = 2,
    #[serde(rename = "error")]
    Error = 3,
    #[serde(rename = "fatal")]
    Fatal = 4,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Note => write!(f, "note"),
            Self::Hint => write!(f, "hint"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
            Self::Fatal => write!(f, "fatal"),
        }
    }
}

/// A suggested fix for a diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Fix {
    /// Replace the range with this string.
    Replace(String),
    /// Insert this string at the start of the range.
    Insert(String),
    /// Delete the range.
    Delete,
    /// Apply a multi-line text patch.
    Patch(String),
}

impl Diagnostic {
    pub fn new(
        rule_id: impl Into<String>,
        message: impl Into<String>,
        severity: Severity,
        file: PathBuf,
        range: Range,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            message: message.into(),
            severity,
            file,
            range,
            fix: None,
            confidence: 1.0,
            explanation: None,
        }
    }

    pub fn with_fix(mut self, fix: Fix) -> Self {
        self.fix = Some(fix);
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn with_explanation(mut self, explanation: impl Into<String>) -> Self {
        self.explanation = Some(explanation.into());
        self
    }

    /// Check if this diagnostic passes a confidence threshold.
    pub fn passes_threshold(&self, threshold: f32) -> bool {
        self.confidence >= threshold
    }
}

/// Summary of a lint run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintSummary {
    pub total_files: usize,
    pub total_diagnostics: usize,
    pub by_severity: std::collections::HashMap<String, usize>,
    pub by_rule: std::collections::HashMap<String, usize>,
    pub duration_ms: u128,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_builder() {
        let range = Range::new(
            Position { line: 5, column: 10 },
            Position { line: 5, column: 20 },
            100,
            110,
        );
        let diag = Diagnostic::new(
            "rule-1",
            "This is an issue",
            Severity::Warning,
            PathBuf::from("main.rs"),
            range,
        )
        .with_confidence(0.95)
        .with_explanation("This violates style guide XYZ");

        assert_eq!(diag.rule_id, "rule-1");
        assert_eq!(diag.confidence, 0.95);
        assert!(diag.passes_threshold(0.9));
        assert!(!diag.passes_threshold(0.99));
    }
}
