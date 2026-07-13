//! Core traits for the Anti-Hallucination Framework
//!
//! Defines the plugin interfaces that allow different implementations of:
//! - Knowledge bases
//! - Verifiers
//! - Bias detectors
//! - Confidence extractors
//! - Arbiter logic

use async_trait::async_trait;
use crate::{
    FactualClaim, GroundingScore, VerificationResult, BiasScore, ConfidenceScore, AhfError,
};

/// Knowledge base trait for fact lookup and validation
#[async_trait]
pub trait KnowledgeBase: Send + Sync {
    /// Look up facts related to a claim
    async fn lookup(&self, claim: &FactualClaim) -> Result<Vec<String>, AhfError>;

    /// Validate a source for trustworthiness
    async fn validate_source(&self, source: &str) -> Result<bool, AhfError>;
}

/// Verifier trait for checking claims against sources
#[async_trait]
pub trait Verifier: Send + Sync {
    /// Verify a claim against available sources
    async fn verify(&self, claim: &FactualClaim) -> Result<VerificationResult, AhfError>;

    /// Check for contradictions with known facts
    async fn check_contradiction(&self, claim: &FactualClaim) -> Result<bool, AhfError>;
}

/// Bias detector trait for identifying biased reasoning
#[async_trait]
pub trait BiasDetector: Send + Sync {
    /// Detect biases in a claim
    async fn detect_bias(&self, claim: &FactualClaim) -> Result<BiasScore, AhfError>;

    /// Analyze bias violations against policy
    async fn analyze_violations(&self, bias: &BiasScore) -> Result<Vec<String>, AhfError>;
}

/// Confidence extractor trait for model calibration
#[async_trait]
pub trait ConfidenceExtractor: Send + Sync {
    /// Extract confidence from model output
    async fn extract(&self, text: &str) -> Result<ConfidenceScore, AhfError>;

    /// Calibrate confidence scores
    async fn calibrate(&self, score: ConfidenceScore) -> Result<ConfidenceScore, AhfError>;
}

/// Arbiter logic trait for making decisions
#[async_trait]
pub trait ArbiterLogic: Send + Sync {
    /// Make a decision based on signals
    async fn decide(
        &self,
        grounding_score: GroundingScore,
        verification_result: VerificationResult,
        model_confidence: ConfidenceScore,
        bias_score: BiasScore,
        criticality: Criticality,
    ) -> Result<crate::AhfDecision, AhfError>;
}

/// Criticality level for claims requiring escalation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Criticality {
    Low,
    Medium,
    High,
    Critical,
}

use serde::{Deserialize, Serialize};
