//! Mock AI Model for Testing
//!
//! Simulates AI model outputs including both correct and hallucinatory responses.
//! Used for integration testing without requiring actual LLM access.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Categories of hallucinations for testing
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum HallucinationCategory {
    /// Fabricated facts that have no basis in reality
    Fabrication,
    /// Direct contradictions to known facts
    Contradiction,
    /// Temporal impossibilities or violations
    TemporalViolation,
    /// Stereotypical or biased characterizations
    Stereotype,
    /// Subtle bias with disparate outcomes
    SubtleBias,
    /// High confidence on low-certainty topics
    ConfidenceMismatch,
    /// Citation of non-existent sources
    FalseAttribution,
    /// Numeric or quantitative errors
    NumericError,
    /// Logical fallacies
    LogicalFallacy,
    /// Context misuse or misapplication
    ContextMisuse,
}

impl std::fmt::Display for HallucinationCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fabrication => write!(f, "Fabrication"),
            Self::Contradiction => write!(f, "Contradiction"),
            Self::TemporalViolation => write!(f, "TemporalViolation"),
            Self::Stereotype => write!(f, "Stereotype"),
            Self::SubtleBias => write!(f, "SubtleBias"),
            Self::ConfidenceMismatch => write!(f, "ConfidenceMismatch"),
            Self::FalseAttribution => write!(f, "FalseAttribution"),
            Self::NumericError => write!(f, "NumericError"),
            Self::LogicalFallacy => write!(f, "LogicalFallacy"),
            Self::ContextMisuse => write!(f, "ContextMisuse"),
        }
    }
}

/// Simulated hallucination output from mock model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HallucinationOutput {
    /// Unique ID for this output
    pub id: Uuid,
    /// The generated text containing hallucination
    pub text: String,
    /// Stated confidence by the model
    pub confidence: f64,
    /// Category of hallucination
    pub category: HallucinationCategory,
    /// Whether this output should be rejected by AHF
    pub should_be_rejected: bool,
    /// Description of what makes this hallucinatory
    pub description: String,
    /// Domain (medical, legal, geographic, etc.)
    pub domain: String,
}

/// Mock AI model for testing
pub struct MockModel {
    outputs: HashMap<String, HallucinationOutput>,
    seed: u64,
}

impl MockModel {
    /// Create a new mock model
    pub fn new() -> Self {
        Self {
            outputs: HashMap::new(),
            seed: 42,
        }
    }

    /// Register a hallucination output
    pub fn register_output(&mut self, prompt: String, output: HallucinationOutput) {
        self.outputs.insert(prompt, output);
    }

    /// Generate response for a prompt
    pub fn generate(&self, prompt: &str) -> Option<HallucinationOutput> {
        self.outputs.get(prompt).cloned()
    }

    /// Get all registered outputs
    pub fn all_outputs(&self) -> Vec<HallucinationOutput> {
        self.outputs.values().cloned().collect()
    }

    /// Get outputs by category
    pub fn outputs_by_category(&self, category: HallucinationCategory) -> Vec<HallucinationOutput> {
        self.outputs
            .values()
            .filter(|o| o.category == category)
            .cloned()
            .collect()
    }

    /// Get outputs by domain
    pub fn outputs_by_domain(&self, domain: &str) -> Vec<HallucinationOutput> {
        self.outputs
            .values()
            .filter(|o| o.domain == domain)
            .cloned()
            .collect()
    }

    /// Create a standard test output
    pub fn create_output(
        text: String,
        confidence: f64,
        category: HallucinationCategory,
        should_be_rejected: bool,
        description: String,
        domain: String,
    ) -> HallucinationOutput {
        HallucinationOutput {
            id: Uuid::new_v4(),
            text,
            confidence,
            category,
            should_be_rejected,
            description,
            domain,
        }
    }
}

impl Default for MockModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_model_creation() {
        let model = MockModel::new();
        assert_eq!(model.all_outputs().len(), 0);
    }

    #[test]
    fn test_register_and_retrieve_output() {
        let mut model = MockModel::new();
        let output = HallucinationOutput {
            id: Uuid::new_v4(),
            text: "Paris is the capital of Germany".to_string(),
            confidence: 0.95,
            category: HallucinationCategory::Contradiction,
            should_be_rejected: true,
            description: "Direct contradiction to known fact".to_string(),
            domain: "geographic".to_string(),
        };

        let prompt = "What is the capital of Germany?".to_string();
        model.register_output(prompt.clone(), output.clone());

        let retrieved = model.generate(&prompt);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().text, "Paris is the capital of Germany");
    }

    #[test]
    fn test_filter_by_category() {
        let mut model = MockModel::new();
        let fab = HallucinationOutput {
            id: Uuid::new_v4(),
            text: "Atlantis exists".to_string(),
            confidence: 0.8,
            category: HallucinationCategory::Fabrication,
            should_be_rejected: true,
            description: "Non-existent place".to_string(),
            domain: "geographic".to_string(),
        };

        let contra = HallucinationOutput {
            id: Uuid::new_v4(),
            text: "Paris is in Germany".to_string(),
            confidence: 0.9,
            category: HallucinationCategory::Contradiction,
            should_be_rejected: true,
            description: "Wrong country".to_string(),
            domain: "geographic".to_string(),
        };

        model.register_output("prompt1".to_string(), fab);
        model.register_output("prompt2".to_string(), contra);

        assert_eq!(model.outputs_by_category(HallucinationCategory::Fabrication).len(), 1);
        assert_eq!(model.outputs_by_category(HallucinationCategory::Contradiction).len(), 1);
    }
}
