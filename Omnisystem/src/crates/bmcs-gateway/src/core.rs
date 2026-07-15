//! Core BMCS (Bonsai Medical Companion System) types: response tiers,
//! classification results, conversation context, and the final assembled
//! response returned to the caller.

use serde::{Deserialize, Serialize};

/// Escalation/response tier assigned by [`crate::safety::ContextClassifier`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseTier {
    /// Immediate life-threatening emergency
    Emergency,
    /// Crisis requiring urgent professional intervention
    Critical,
    /// Significant distress warranting professional guidance
    Elevated,
    /// Manageable distress
    Moderate,
    /// General informational query
    Low,
    /// Uncertain — safest generic fallback
    Fallback,
}

/// Result of classifying a query (and optional context) into a response tier
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub tier: ResponseTier,
    pub confidence: f32,
    pub reasoning: String,
}

/// A single message in the conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Biometric/vitals data, if available from a connected device
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Vitals {
    pub heart_rate: Option<u32>,
    pub consciousness: Option<bool>,
}

/// Context passed alongside a query: conversation history and any vitals
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BMCSContext {
    pub conversation_history: Option<Vec<Message>>,
    pub vitals: Option<Vitals>,
}

/// The final, fully-assembled BMCS response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BMCSResponse {
    pub response: String,
    pub disclaimer: String,
    pub confidence: f32,
    pub escalated: bool,
    pub resources: Vec<String>,
    pub tier: String,
    pub sources: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_tier_equality() {
        assert_eq!(ResponseTier::Emergency, ResponseTier::Emergency);
        assert_ne!(ResponseTier::Emergency, ResponseTier::Low);
    }

    #[test]
    fn test_context_default_is_empty() {
        let ctx = BMCSContext::default();
        assert!(ctx.vitals.is_none());
        assert!(ctx.conversation_history.is_none());
    }
}
