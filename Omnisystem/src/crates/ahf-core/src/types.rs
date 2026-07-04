//! Core types for the Anti-Hallucination Framework
//!
//! Defines fundamental data structures representing factual claims, grounding scores,
//! verification status, confidence levels, and bias detection results.

use crate::error::AhfError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Subject of a factual claim (entity being described).
///
/// Represents the subject in a subject-predicate-object triple.
/// Can be a named entity, URI, or identifier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Subject {
    /// Unique identifier for the subject
    pub id: String,
    /// Human-readable name or label
    pub label: String,
    /// Optional URI for linked data compatibility
    pub uri: Option<String>,
}

impl Subject {
    /// Create a new subject with identifier and label.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            uri: None,
        }
    }

    /// Add a URI to the subject for linked data.
    pub fn with_uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = Some(uri.into());
        self
    }
}

/// Predicate of a factual claim (relationship or property).
///
/// Represents the relationship/property in a subject-predicate-object triple.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Predicate {
    /// Unique identifier for the predicate
    pub id: String,
    /// Human-readable name or label
    pub label: String,
    /// Optional URI for linked data compatibility
    pub uri: Option<String>,
}

impl Predicate {
    /// Create a new predicate with identifier and label.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            uri: None,
        }
    }

    /// Add a URI to the predicate for linked data.
    pub fn with_uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = Some(uri.into());
        self
    }
}

/// A factual claim to be verified by the Anti-Hallucination Framework.
///
/// Represents a subject-predicate-object triple with optional context and metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FactualClaim {
    /// Unique identifier for this claim
    pub id: Uuid,
    /// The subject of the claim
    pub subject: Subject,
    /// The predicate/relationship of the claim
    pub predicate: Predicate,
    /// The object (value/claim being made)
    pub object: String,
    /// Optional context for the claim (e.g., temporal, spatial, conditional)
    pub context: Option<String>,
    /// Confidence in the claim (0.0..1.0) - from the model/source
    pub source_confidence: f64,
    /// Timestamp when the claim was made
    pub timestamp: DateTime<Utc>,
    /// Optional source reference (URL, document ID, etc.)
    pub source_reference: Option<String>,
}

impl FactualClaim {
    /// Create a new factual claim with required fields.
    ///
    /// # Arguments
    /// * `subject` - The entity the claim is about
    /// * `predicate` - The relationship/property being claimed
    /// * `object` - The value/description being claimed
    /// * `source_confidence` - Initial confidence in the claim (0.0..1.0)
    pub fn new(
        subject: Subject,
        predicate: Predicate,
        object: impl Into<String>,
        source_confidence: f64,
    ) -> Result<Self, AhfError> {
        if !(0.0..=1.0).contains(&source_confidence) {
            return Err(AhfError::invalid_configuration(
                "source_confidence must be in range [0.0, 1.0]",
            ));
        }

        Ok(Self {
            id: Uuid::new_v4(),
            subject,
            predicate,
            object: object.into(),
            context: None,
            source_confidence,
            timestamp: Utc::now(),
            source_reference: None,
        })
    }

    /// Add context to the claim.
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Add a source reference to the claim.
    pub fn with_source_reference(mut self, reference: impl Into<String>) -> Self {
        self.source_reference = Some(reference.into());
        self
    }
}

/// Verification status of a claim
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationStatus {
    /// Claim has been verified as true
    Valid,
    /// Claim has been verified as false
    Invalid,
    /// Claim verification is inconclusive
    Inconclusive,
    /// Claim requires manual review
    NeedsReview,
}

/// Grounding score for a factual claim (0.0..1.0).
///
/// Represents how well a claim is grounded in external knowledge sources.
/// Higher scores indicate stronger evidence from reliable sources.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct GroundingScore {
    /// Grounding score in range [0.0, 1.0]
    score: f64,
}

impl GroundingScore {
    /// Create a new grounding score.
    ///
    /// # Errors
    /// Returns `InvalidConfiguration` if score is not in [0.0, 1.0].
    pub fn new(verified: usize, total: usize) -> Self {
        let score = if total == 0 {
            1.0 // Empty claims = fully grounded
        } else {
            verified as f64 / total as f64
        };
        Self {
            score: score.clamp(0.0, 1.0),
        }
    }

    /// Get the numeric score value.
    pub fn score(&self) -> f64 {
        self.score
    }

    /// Check if grounding meets a threshold.
    pub fn meets_threshold(&self, threshold: f64) -> bool {
        self.score >= threshold
    }

    /// Get grounding quality level (Low, Medium, High, Very High).
    pub fn quality_level(&self) -> &'static str {
        match self.score {
            s if s >= 0.9 => "Very High",
            s if s >= 0.7 => "High",
            s if s >= 0.5 => "Medium",
            _ => "Low",
        }
    }

    /// Create a contradicted grounding score
    pub fn contradicted() -> Self {
        Self { score: 0.0 }
    }
}

impl std::fmt::Display for GroundingScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}", self.score)
    }
}

/// Confidence score for a claim (0.0..1.0).
///
/// Represents the model's confidence in the claim after verification.
/// Should be calibrated against actual verification outcomes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ConfidenceScore(pub f64);

impl ConfidenceScore {
    /// Create a new confidence score.
    ///
    /// Clamps the value to [0.0, 1.0].
    pub fn new(score: f64) -> Self {
        Self(score.clamp(0.0, 1.0))
    }

    /// Get the numeric score value.
    pub fn score(&self) -> f64 {
        self.0
    }

    /// Check if confidence meets a threshold.
    pub fn meets_threshold(&self, threshold: f64) -> bool {
        self.0 >= threshold
    }

    /// Check if score indicates full confidence.
    pub fn is_certain(&self) -> bool {
        (self.0 - 1.0).abs() < f64::EPSILON
    }

    /// Check if score indicates no confidence.
    pub fn is_zero(&self) -> bool {
        self.0 < f64::EPSILON
    }

    /// Calculate calibration error against ground truth.
    ///
    /// Returns absolute difference between confidence and outcome (0.0 = perfectly calibrated).
    pub fn calibration_error(&self, is_correct: bool) -> f64 {
        let ground_truth = if is_correct { 1.0 } else { 0.0 };
        (self.0 - ground_truth).abs()
    }
}

impl std::fmt::Display for ConfidenceScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subject_creation() {
        let subject = Subject::new("wiki:Albert_Einstein", "Albert Einstein");
        assert_eq!(subject.id, "wiki:Albert_Einstein");
        assert_eq!(subject.label, "Albert Einstein");
        assert_eq!(subject.uri, None);
    }

    #[test]
    fn test_subject_with_uri() {
        let subject = Subject::new("wiki:Albert_Einstein", "Albert Einstein")
            .with_uri("http://dbpedia.org/resource/Albert_Einstein");
        assert_eq!(subject.uri, Some("http://dbpedia.org/resource/Albert_Einstein".to_string()));
    }

    #[test]
    fn test_predicate_creation() {
        let predicate = Predicate::new("born_in", "Born In");
        assert_eq!(predicate.id, "born_in");
        assert_eq!(predicate.label, "Born In");
    }

    #[test]
    fn test_factual_claim_creation() {
        let subject = Subject::new("wiki:Albert_Einstein", "Albert Einstein");
        let predicate = Predicate::new("born_in", "Born In");
        let claim = FactualClaim::new(subject, predicate, "Ulm, Germany", 0.95).unwrap();

        assert_eq!(claim.object, "Ulm, Germany");
        assert_eq!(claim.source_confidence, 0.95);
        assert!(claim.context.is_none());
    }

    #[test]
    fn test_factual_claim_with_context() {
        let subject = Subject::new("wiki:Albert_Einstein", "Albert Einstein");
        let predicate = Predicate::new("born_in", "Born In");
        let claim = FactualClaim::new(subject, predicate, "Ulm, Germany", 0.95)
            .unwrap()
            .with_context("historical fact from 1879");

        assert_eq!(claim.context, Some("historical fact from 1879".to_string()));
    }

    #[test]
    fn test_factual_claim_invalid_confidence() {
        let subject = Subject::new("test_subject", "Test");
        let predicate = Predicate::new("test_pred", "Test");
        let result = FactualClaim::new(subject, predicate, "test_object", 1.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_grounding_score_creation() {
        let score = GroundingScore::new(3, 5);
        assert!((score.score() - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn test_grounding_score_empty() {
        let score = GroundingScore::new(0, 0);
        assert_eq!(score.score(), 1.0); // Empty = fully grounded
    }

    #[test]
    fn test_grounding_score_meets_threshold() {
        let score = GroundingScore::new(7, 10);
        assert!(score.meets_threshold(0.7));
        assert!(!score.meets_threshold(0.8));
    }

    #[test]
    fn test_grounding_quality_levels() {
        assert_eq!(GroundingScore::new(9, 10).quality_level(), "Very High");
        assert_eq!(GroundingScore::new(7, 10).quality_level(), "High");
        assert_eq!(GroundingScore::new(5, 10).quality_level(), "Medium");
        assert_eq!(GroundingScore::new(3, 10).quality_level(), "Low");
    }

    #[test]
    fn test_confidence_score_clamp() {
        assert_eq!(ConfidenceScore::new(1.5).0, 1.0);
        assert_eq!(ConfidenceScore::new(-0.5).0, 0.0);
        assert_eq!(ConfidenceScore::new(0.5).0, 0.5);
    }

    #[test]
    fn test_confidence_score_checks() {
        let certain = ConfidenceScore::new(1.0);
        assert!(certain.is_certain());
        assert!(!certain.is_zero());

        let zero = ConfidenceScore::new(0.0);
        assert!(zero.is_zero());
        assert!(!zero.is_certain());

        assert!(ConfidenceScore::new(0.8).meets_threshold(0.7));
        assert!(!ConfidenceScore::new(0.6).meets_threshold(0.7));
    }

    #[test]
    fn test_confidence_calibration_error() {
        let score = ConfidenceScore::new(0.8);
        assert!((score.calibration_error(true) - 0.2).abs() < f64::EPSILON);
        assert!((score.calibration_error(false) - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_verification_status_serialization() {
        let status = VerificationStatus::Valid;
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: VerificationStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, deserialized);
    }

    #[test]
    fn test_grounding_score_contradicted() {
        let score = GroundingScore::contradicted();
        assert_eq!(score.score(), 0.0);
    }
}
