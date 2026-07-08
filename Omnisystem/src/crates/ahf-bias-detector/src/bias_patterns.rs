//! Deterministic bias pattern detection using regex patterns
//!
//! This module provides fast, offline bias detection using pre-compiled regex patterns.
//! Patterns are loaded from the UMS module `bias-patterns-v1` and cover common stereotypes,
//! demographic disparities, and generalizations.
//!
//! ## Patterns
//!
//! - **Stereotypical Generalizations**: "all [group] are [stereotype]"
//! - **Demographic Disparities**: Unequal treatment or outcomes based on demographics
//! - **Loaded Language**: Emotional or charged terminology
//! - **False Causation**: Implying causation without evidence
//! - **Hasty Generalization**: Sweeping claims from limited evidence

use crate::error::BiasDetectorError;
use serde::{Deserialize, Serialize};
use regex::Regex;
use uuid::Uuid;

/// Severity level of a bias violation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BiasLevel {
    /// Low severity bias, minimal impact
    Low = 1,
    /// Medium severity bias, noticeable impact
    Medium = 2,
    /// High severity bias, significant impact
    High = 3,
    /// Critical severity bias, severe impact
    Critical = 4,
}

impl BiasLevel {
    /// Get numeric score (1-4)
    pub fn score(&self) -> u8 {
        *self as u8
    }

    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            BiasLevel::Low => "Low severity: minimal impact",
            BiasLevel::Medium => "Medium severity: noticeable impact",
            BiasLevel::High => "High severity: significant impact",
            BiasLevel::Critical => "Critical severity: severe impact",
        }
    }
}

/// A bias detection pattern with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasPattern {
    /// Unique identifier
    pub id: Uuid,
    /// Human-readable name
    pub name: String,
    /// Regex pattern for matching
    #[serde(skip)]
    pub regex: Option<Regex>,
    /// Raw regex string for serialization
    pub pattern: String,
    /// Description of the bias type
    pub description: String,
    /// Default severity level
    pub severity: BiasLevel,
    /// Category (e.g., "stereotype", "demographic_disparity")
    pub category: String,
    /// Examples of matching text
    pub examples: Vec<String>,
}

impl BiasPattern {
    /// Create a new bias pattern
    pub fn new(
        name: impl Into<String>,
        pattern: impl Into<String>,
        description: impl Into<String>,
        severity: BiasLevel,
        category: impl Into<String>,
    ) -> Result<Self, BiasDetectorError> {
        let pattern_str = pattern.into();
        let regex = Regex::new(&pattern_str)
            .map_err(|e| BiasDetectorError::pattern_matching_failed(e.to_string()))?;

        Ok(BiasPattern {
            id: Uuid::new_v4(),
            name: name.into(),
            regex: Some(regex),
            pattern: pattern_str,
            description: description.into(),
            severity,
            category: category.into(),
            examples: Vec::new(),
        })
    }

    /// Add example text that matches this pattern
    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }

    /// Compile the pattern string into a regex
    pub fn compile(&mut self) -> Result<(), BiasDetectorError> {
        let regex = Regex::new(&self.pattern)
            .map_err(|e| BiasDetectorError::pattern_matching_failed(e.to_string()))?;
        self.regex = Some(regex);
        Ok(())
    }

    /// Check if a text matches this pattern
    pub fn matches(&self, text: &str) -> bool {
        if let Some(ref regex) = self.regex {
            regex.is_match(text)
        } else {
            false
        }
    }

    /// Find all matches in text with positions
    pub fn find_matches(&self, text: &str) -> Vec<(usize, usize, String)> {
        if let Some(ref regex) = self.regex {
            regex
                .find_iter(text)
                .map(|m| (m.start(), m.end(), m.as_str().to_string()))
                .collect()
        } else {
            Vec::new()
        }
    }
}

/// A bias violation found in text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasViolation {
    /// Unique identifier
    pub id: Uuid,
    /// Pattern that matched
    pub pattern: BiasPattern,
    /// Severity of violation
    pub severity: BiasLevel,
    /// Position in text where violation was found
    pub position: (usize, usize), // (start, end)
    /// Matched text
    pub matched_text: String,
    /// Explanation of the violation
    pub explanation: String,
}

impl BiasViolation {
    /// Create a new bias violation
    pub fn new(
        pattern: BiasPattern,
        position: (usize, usize),
        matched_text: impl Into<String>,
    ) -> Self {
        let matched_str = matched_text.into();
        let explanation = format!(
            "Pattern '{}' matched: {}. Category: {}. {}",
            pattern.name,
            pattern.description,
            pattern.category,
            pattern.severity.description()
        );

        BiasViolation {
            id: Uuid::new_v4(),
            severity: pattern.severity,
            pattern,
            position,
            matched_text: matched_str,
            explanation,
        }
    }

    /// Get character range as text slice from original
    pub fn context(&self, original_text: &str, context_chars: usize) -> String {
        let start = self.position.0.saturating_sub(context_chars);
        let end = (self.position.1 + context_chars).min(original_text.len());
        original_text[start..end].to_string()
    }
}

/// Pattern matcher that applies bias patterns to text
pub struct PatternMatcher {
    patterns: Vec<BiasPattern>,
}

impl PatternMatcher {
    /// Create a new pattern matcher with default built-in patterns
    pub fn new() -> Result<Self, BiasDetectorError> {
        let patterns = Self::default_patterns()?;
        Ok(PatternMatcher { patterns })
    }

    /// Create a pattern matcher with custom patterns
    pub fn with_patterns(patterns: Vec<BiasPattern>) -> Self {
        PatternMatcher { patterns }
    }

    /// Add a pattern to the matcher
    pub fn add_pattern(&mut self, pattern: BiasPattern) {
        self.patterns.push(pattern);
    }

    /// Get all patterns
    pub fn patterns(&self) -> &[BiasPattern] {
        &self.patterns
    }

    /// Match all patterns against text
    pub fn match_all(&self, text: &str) -> Vec<BiasViolation> {
        let mut violations = Vec::new();

        for pattern in &self.patterns {
            for (start, end, matched) in pattern.find_matches(text) {
                violations.push(BiasViolation::new(
                    pattern.clone(),
                    (start, end),
                    matched,
                ));
            }
        }

        // Sort by severity (highest first) then by position
        violations.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then(a.position.0.cmp(&b.position.0))
        });

        violations
    }

    /// Match patterns and return only violations at or above minimum severity
    pub fn match_with_severity(
        &self,
        text: &str,
        min_severity: BiasLevel,
    ) -> Vec<BiasViolation> {
        self.match_all(text)
            .into_iter()
            .filter(|v| v.severity >= min_severity)
            .collect()
    }

    /// Get maximum severity violation in text
    pub fn max_severity(&self, text: &str) -> Option<BiasLevel> {
        self.match_all(text)
            .into_iter()
            .map(|v| v.severity)
            .max()
    }

    /// Check if text contains any bias violations
    pub fn contains_bias(&self, text: &str) -> bool {
        self.patterns.iter().any(|p| p.matches(text))
    }

    /// Build default patterns from UMS module `bias-patterns-v1`
    fn default_patterns() -> Result<Vec<BiasPattern>, BiasDetectorError> {
        let patterns = vec![
            // Stereotypical generalizations
            BiasPattern::new(
                "all-group-stereotype",
                r"(?i)\ball\s+(people|men|women|groups?)\s+(of|from|in)?\s*([\w\s]+)\s+(are|is)\s+([\w\s]+)",
                "Stereotypical generalization: 'all [group] are [stereotype]'",
                BiasLevel::High,
                "stereotype",
            )?
            .with_example("all women are emotional")
            .with_example("all men from that country are aggressive"),

            // Demographic disparities in language
            BiasPattern::new(
                "demographic-disparity",
                r"(?i)\b((?:never|always|only|just)\s+)?(people|those|folks?|individuals?)\s+(?:of|from|with|named)\s+([\w\s]+)\s+(?:are|can't|won't|don't)\s+",
                "Potential demographic disparity in language",
                BiasLevel::Medium,
                "demographic_disparity",
            )?
            .with_example("people from that region always...")
            .with_example("those folks with that name never..."),

            // Inherently negative group associations
            BiasPattern::new(
                "negative-association",
                r"(?i)\b([\w\s]+)\s+(is|are)\s+(criminal|lazy|dumb|stupid|savage|primitive|uncivilized)\b",
                "Inherent negative association with a group",
                BiasLevel::Critical,
                "stereotype",
            )?
            .with_example("that group is naturally lazy")
            .with_example("people from there are savage"),

            // False causation and stereotypes
            BiasPattern::new(
                "false-causation",
                r"(?i)because\s+(they|they're|he's|she's|it's)\s+([\w\s]+)\s*,\s*(?:they|he|she|it)\s+([\w\s]+)",
                "False causation linking group characteristics to outcomes",
                BiasLevel::High,
                "false_causation",
            )?
            .with_example("because they're that nationality, they must be...")
            .with_example("because she's a woman, she can't..."),

            // Hasty generalization with quantifiers
            BiasPattern::new(
                "hasty-generalization",
                r"(?i)\b(most|many|some|few|all|several)\s+([\w\s]+)\s+(are|is|tend|seem)\s+",
                "Hasty generalization making sweeping claims",
                BiasLevel::Medium,
                "generalization",
            )?
            .with_example("most people from that background are...")
            .with_example("all those types of people seem..."),

            // Coded language patterns
            BiasPattern::new(
                "coded-language",
                r"(?i)\b(thug|articulate|inner-city|urban|thugs?|welfare|welfare queen)\b",
                "Potentially coded language with historical bias associations",
                BiasLevel::Medium,
                "coded_language",
            )?
            .with_example("he's so articulate for...")
            .with_example("those urban communities..."),

            // Essentialist language
            BiasPattern::new(
                "essentialist",
                r"(?i)\b(naturally|inherently|by nature|fundamentally)\s+(?:[\w\s]+\s+)?(is|are|can|can't|will|won't)\s+",
                "Essentialist claims about groups or identities",
                BiasLevel::High,
                "essentialism",
            )?
            .with_example("women are naturally more nurturing")
            .with_example("people from that group are inherently..."),

            // Loaded language with strong emotional words
            BiasPattern::new(
                "loaded-language",
                r"(?i)\b([\w\s]+)\s+(infest|plague|invade|swarm)\s+",
                "Dehumanizing or emotionally charged language",
                BiasLevel::Critical,
                "loaded_language",
            )?
            .with_example("they infest the neighborhoods")
            .with_example("immigrants swarm the border"),

            // Implicit bias in framing
            BiasPattern::new(
                "framing-bias",
                r"(?i)(?:interestingly|surprisingly|remarkably)\s+(?:enough|,)\s+([\w\s]+)\s+([\w\s]+)?\s+(is|are|do|does)\s+",
                "Framing suggests surprise at capabilities, implies low baseline expectations",
                BiasLevel::Medium,
                "framing",
            )?
            .with_example("surprisingly, she's quite intelligent")
            .with_example("remarkably, they speak fluent English"),

            // Othering language
            BiasPattern::new(
                "othering",
                r"(?i)\bthose\s+(?:people|types|kind|sort)\s+(?:of|from|with|named)\s+",
                "Othering language that creates distance or separates groups",
                BiasLevel::Medium,
                "othering",
            )?
            .with_example("those people from...")
            .with_example("those types of folks..."),
        ];

        Ok(patterns)
    }
}

impl Default for PatternMatcher {
    fn default() -> Self {
        // Constructor shouldn't fail for default patterns; unwrap if it does
        Self::new().expect("Failed to create default PatternMatcher")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bias_level_score() {
        assert_eq!(BiasLevel::Low.score(), 1);
        assert_eq!(BiasLevel::Medium.score(), 2);
        assert_eq!(BiasLevel::High.score(), 3);
        assert_eq!(BiasLevel::Critical.score(), 4);
    }

    #[test]
    fn test_bias_level_ordering() {
        assert!(BiasLevel::Low < BiasLevel::Medium);
        assert!(BiasLevel::Medium < BiasLevel::High);
        assert!(BiasLevel::High < BiasLevel::Critical);
    }

    #[test]
    fn test_bias_pattern_creation() {
        let pattern = BiasPattern::new(
            "test",
            r"test pattern",
            "A test pattern",
            BiasLevel::Medium,
            "test",
        )
        .expect("Failed to create pattern");
        assert_eq!(pattern.name, "test");
        assert!(pattern.matches("this is a test pattern"));
    }

    #[test]
    fn test_bias_pattern_with_example() {
        let pattern = BiasPattern::new(
            "test",
            r"example",
            "Test",
            BiasLevel::Low,
            "test",
        )
        .expect("Failed to create pattern")
        .with_example("example 1")
        .with_example("example 2");

        assert_eq!(pattern.examples.len(), 2);
    }

    #[test]
    fn test_pattern_matcher_default() {
        let matcher = PatternMatcher::default();
        assert!(!matcher.patterns().is_empty());
    }

    #[test]
    fn test_stereotype_pattern_detection() {
        let matcher = PatternMatcher::default();
        let text = "all women are emotional";
        assert!(matcher.contains_bias(text));
        let violations = matcher.match_all(text);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_negative_association_detection() {
        let matcher = PatternMatcher::default();
        let text = "people from that group are savage";
        let violations = matcher.match_all(text);
        assert!(!violations.is_empty());
        assert_eq!(violations[0].severity, BiasLevel::Critical);
    }

    #[test]
    fn test_match_with_severity_filtering() {
        let matcher = PatternMatcher::default();
        let text = "all women are emotional and people from there are primitive";
        let violations = matcher.match_with_severity(text, BiasLevel::High);
        assert!(!violations.is_empty());
        for violation in violations {
            assert!(violation.severity >= BiasLevel::High);
        }
    }

    #[test]
    fn test_max_severity() {
        let matcher = PatternMatcher::default();
        let text = "all women are emotional and people from there are savage"; // Critical
        let max = matcher.max_severity(text);
        assert!(max.is_some());
        assert!(max.unwrap() >= BiasLevel::High);
    }

    #[test]
    fn test_bias_violation_context() {
        let pattern = BiasPattern::new(
            "test",
            r"bad",
            "Test",
            BiasLevel::Medium,
            "test",
        )
        .expect("Failed to create pattern");
        let violation = BiasViolation::new(pattern, (10, 13), "bad");
        let text = "This is really bad text here";
        let context = violation.context(text, 5);
        assert!(context.contains("bad"));
    }

    #[test]
    fn test_no_bias_in_neutral_text() {
        let matcher = PatternMatcher::default();
        let text = "The weather is nice today and the sky is blue.";
        assert!(!matcher.contains_bias(text));
        let violations = matcher.match_all(text);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_pattern_find_matches_positions() {
        let pattern = BiasPattern::new(
            "test",
            r"test",
            "Test",
            BiasLevel::Low,
            "test",
        )
        .expect("Failed to create pattern");
        let text = "test one test two test three";
        let matches = pattern.find_matches(text);
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].0, 0);
        assert_eq!(matches[1].0, 9);
        assert_eq!(matches[2].0, 18);
    }

    #[test]
    fn test_custom_pattern_matcher() {
        let pattern = BiasPattern::new(
            "custom",
            r"(?i)forbidden",
            "Custom forbidden word",
            BiasLevel::High,
            "custom",
        )
        .expect("Failed to create pattern");
        let matcher = PatternMatcher::with_patterns(vec![pattern]);
        assert!(matcher.contains_bias("This is forbidden"));
        assert!(!matcher.contains_bias("This is allowed"));
    }

    #[test]
    fn test_violation_severity_sorting() {
        let matcher = PatternMatcher::default();
        let text = "all women are emotional and people from there are savage";
        let violations = matcher.match_all(text);
        // Should be sorted by severity (Critical before High)
        if violations.len() > 1 {
            assert!(violations[0].severity >= violations[1].severity);
        }
    }

    #[test]
    fn test_coded_language_detection() {
        let matcher = PatternMatcher::default();
        let text = "he's so articulate for someone from the inner-city";
        let violations = matcher.match_all(text);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_othering_language_detection() {
        let matcher = PatternMatcher::default();
        let text = "those people from that country are...";
        let violations = matcher.match_all(text);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_add_pattern_to_matcher() {
        let mut matcher = PatternMatcher::new().expect("Failed to create matcher");
        let custom_pattern = BiasPattern::new(
            "custom",
            r"(?i)xyz",
            "Custom pattern",
            BiasLevel::Medium,
            "custom",
        )
        .expect("Failed to create pattern");
        let initial_count = matcher.patterns().len();
        matcher.add_pattern(custom_pattern);
        assert_eq!(matcher.patterns().len(), initial_count + 1);
    }

    #[test]
    fn test_hasty_generalization_detection() {
        let matcher = PatternMatcher::default();
        let text = "most people from that background are lazy";
        let violations = matcher.match_all(text);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_essentialist_language_detection() {
        let matcher = PatternMatcher::default();
        let text = "Women naturally are more nurturing than men";
        let violations = matcher.match_all(text);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_dehumanizing_language_detection() {
        let matcher = PatternMatcher::default();
        let text = "immigrants infest our neighborhoods";
        let violations = matcher.match_all(text);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_false_causation_detection() {
        let matcher = PatternMatcher::default();
        let text = "Because she's a woman, she can't be a good engineer";
        let violations = matcher.match_all(text);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_pattern_compile() {
        let mut pattern = BiasPattern {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            regex: None,
            pattern: r"test".to_string(),
            description: "Test".to_string(),
            severity: BiasLevel::Low,
            category: "test".to_string(),
            examples: vec![],
        };
        assert!(pattern.regex.is_none());
        pattern.compile().expect("Failed to compile");
        assert!(pattern.regex.is_some());
    }
}
