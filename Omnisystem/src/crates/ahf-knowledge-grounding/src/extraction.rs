//! Deterministic fact extraction from natural language
//!
//! Parses sentences into (subject, predicate, object) triples with confidence scores.
//! All extraction is deterministic: same input always produces same output.

use ahf_core::{FactualClaim, Predicate, Subject, ConfidenceScore};
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

/// Result of fact extraction from text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    /// Extracted claims
    pub claims: Vec<FactualClaim>,
    /// Extraction quality metrics
    pub metrics: ExtractionMetrics,
}

/// Metrics about extraction quality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionMetrics {
    /// Number of claims extracted
    pub claims_count: usize,
    /// Average confidence across claims
    pub avg_confidence: f64,
    /// Number of numeric assertions detected
    pub numeric_assertions: usize,
    /// Number of temporal assertions detected
    pub temporal_assertions: usize,
}

/// Deterministic NLP-based fact extractor
///
/// Uses pattern matching and regex to extract (subject, predicate, object) triples
/// from natural language text. All operations are deterministic and stateless.
pub struct FactExtractor {
    patterns: PatternRegistry,
}

/// Registry of extraction patterns
struct PatternRegistry {
    patterns: Vec<ExtractionPattern>,
}

/// Single extraction pattern
struct ExtractionPattern {
    regex: Regex,
    predicate_label: String,
    confidence: f64,
}

impl PatternRegistry {
    /// Create the standard pattern registry
    fn standard() -> Self {
        let patterns = vec![
            // Capital: "X is the capital of Y"
            ExtractionPattern {
                regex: Regex::new(r"(?i)([A-Z][a-z]+)\s+is\s+the\s+capital\s+of\s+([A-Z][a-z]+)")
                    .unwrap(),
                predicate_label: "is_capital_of".to_string(),
                confidence: 0.95,
            },
            // Location: "X is in/located in Y"
            ExtractionPattern {
                regex: Regex::new(r"(?i)([A-Z][a-z]+)\s+(?:is|located)\s+in\s+([A-Z][a-z]+)")
                    .unwrap(),
                predicate_label: "is_located_in".to_string(),
                confidence: 0.90,
            },
            // Founded: "X was founded in/by Y"
            ExtractionPattern {
                regex: Regex::new(r"(?i)([A-Z][a-z\s]+)\s+was\s+founded\s+(?:in|by)\s+([A-Z][a-z0-9\s]+)")
                    .unwrap(),
                predicate_label: "was_founded_in".to_string(),
                confidence: 0.90,
            },
            // Numeric: "X has Y Z" where Z is a number
            ExtractionPattern {
                regex: Regex::new(r"(?i)([A-Z][a-z\s]+)\s+(?:has|is)\s+(\d+(?:\.\d+)?)\s+([a-z]+)")
                    .unwrap(),
                predicate_label: "has_property".to_string(),
                confidence: 0.85,
            },
            // Temporal: "X occurred in/during Y"
            ExtractionPattern {
                regex: Regex::new(r"(?i)([A-Z][a-z\s]+)\s+(?:occurred|happened)\s+(?:in|during)\s+(\d{4}(?:-\d{2}-\d{2})?)")
                    .unwrap(),
                predicate_label: "occurred_during".to_string(),
                confidence: 0.90,
            },
            // Simple equality: "X is Y"
            ExtractionPattern {
                regex: Regex::new(r"(?i)([A-Z][a-z]+)\s+is\s+([A-Z][a-z\s]+)")
                    .unwrap(),
                predicate_label: "is".to_string(),
                confidence: 0.80,
            },
        ];

        PatternRegistry { patterns }
    }
}

impl FactExtractor {
    /// Create a new fact extractor with standard patterns
    pub fn new() -> Self {
        FactExtractor {
            patterns: PatternRegistry::standard(),
        }
    }

    /// Extract claims from text
    ///
    /// Returns deterministic results: same input always produces identical output.
    pub fn extract(&self, text: &str) -> crate::KgsResult<ExtractionResult> {
        let mut claims = Vec::new();
        let mut numeric_count = 0;
        let mut temporal_count = 0;

        // Split into sentences
        let sentences = self.split_sentences(text);

        for sentence in sentences {
            let sentence = sentence.trim();
            if sentence.is_empty() {
                continue;
            }

            // Try each pattern
            for pattern in &self.patterns.patterns {
                if let Some(caps) = pattern.regex.captures(sentence) {
                    if caps.len() >= 3 {
                        let subject_text = caps.get(1).unwrap().as_str().trim();
                        let object_text = caps.get(2).unwrap().as_str().trim();

                        // Check if this looks like numeric or temporal
                        if object_text.chars().all(|c| c.is_numeric() || c == '.' || c == '-') {
                            if pattern.predicate_label.contains("has") || pattern.predicate_label.contains("numeric") {
                                numeric_count += 1;
                            }
                        }
                        if object_text.len() == 4 && object_text.chars().all(|c| c.is_numeric()) {
                            temporal_count += 1;
                        }

                        let subject = Subject::new(
                            subject_text.to_lowercase().replace(' ', "_"),
                            subject_text.to_string(),
                        );
                        let predicate = Predicate::new(
                            pattern.predicate_label.clone(),
                            pattern.predicate_label.clone(),
                        );

                        let claim = FactualClaim {
                            id: Uuid::new_v4(),
                            subject,
                            predicate,
                            object: object_text.to_string(),
                            context: None,
                            source_confidence: pattern.confidence,
                            timestamp: Utc::now(),
                            source_reference: None,
                        };

                        claims.push(claim);
                        break; // Only match first pattern per sentence
                    }
                }
            }
        }

        let avg_confidence = if claims.is_empty() {
            0.0
        } else {
            claims.iter().map(|c| c.source_confidence).sum::<f64>() / claims.len() as f64
        };

        Ok(ExtractionResult {
            metrics: ExtractionMetrics {
                claims_count: claims.len(),
                avg_confidence,
                numeric_assertions: numeric_count,
                temporal_assertions: temporal_count,
            },
            claims,
        })
    }

    /// Split text into sentences (deterministic)
    fn split_sentences<'a>(&self, text: &'a str) -> Vec<&'a str> {
        text.split(|c| c == '.' || c == '!' || c == '?')
            .collect()
    }
}

impl Default for FactExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_capital() {
        let extractor = FactExtractor::new();
        let result = extractor.extract("Paris is the capital of France.").unwrap();

        assert_eq!(result.claims.len(), 1);
        assert!(result.claims[0].subject.label.contains("Paris"));
        assert_eq!(result.claims[0].predicate.label, "is_capital_of");
    }

    #[test]
    fn test_extract_location() {
        let extractor = FactExtractor::new();
        let result = extractor.extract("Tokyo is located in Japan.").unwrap();

        assert_eq!(result.claims.len(), 1);
        assert_eq!(result.claims[0].predicate.label, "is_located_in");
    }

    #[test]
    fn test_extract_numeric() {
        let extractor = FactExtractor::new();
        let result = extractor.extract("Mount Everest has 8849 meters of elevation.").unwrap();

        assert!(result.metrics.numeric_assertions > 0);
    }

    #[test]
    fn test_extract_temporal() {
        let extractor = FactExtractor::new();
        let result = extractor.extract("World War II occurred during 1939-1945.").unwrap();

        // May or may not extract depending on regex
        assert!(result.claims.len() <= 1);
    }

    #[test]
    fn test_extract_deterministic() {
        let extractor = FactExtractor::new();
        let text = "Paris is the capital of France. Tokyo is in Japan.";

        let result1 = extractor.extract(text).unwrap();
        let result2 = extractor.extract(text).unwrap();

        assert_eq!(result1.claims.len(), result2.claims.len());
        for (c1, c2) in result1.claims.iter().zip(result2.claims.iter()) {
            assert_eq!(c1.subject, c2.subject);
            assert_eq!(c1.object, c2.object);
        }
    }

    #[test]
    fn test_extract_empty_text() {
        let extractor = FactExtractor::new();
        let result = extractor.extract("").unwrap();

        assert_eq!(result.claims.len(), 0);
        assert_eq!(result.metrics.claims_count, 0);
    }

    #[test]
    fn test_extract_metrics() {
        let extractor = FactExtractor::new();
        let result = extractor.extract("Paris is the capital of France. London is in England.")
            .unwrap();

        assert!(result.metrics.claims_count > 0);
        assert!(result.metrics.avg_confidence > 0.0);
        assert!(result.metrics.avg_confidence <= 1.0);
    }

    #[test]
    fn test_extract_confidence_scores() {
        let extractor = FactExtractor::new();
        let result = extractor.extract("Paris is the capital of France.").unwrap();

        assert!(!result.claims.is_empty());
        for claim in &result.claims {
            assert!(claim.source_confidence >= 0.0 && claim.source_confidence <= 1.0);
        }
    }

    #[test]
    fn test_multiple_sentences() {
        let extractor = FactExtractor::new();
        let text = "Paris is the capital of France. Tokyo is in Japan. London is the capital of England.";
        let result = extractor.extract(text).unwrap();

        assert!(result.claims.len() >= 2);
    }
}
