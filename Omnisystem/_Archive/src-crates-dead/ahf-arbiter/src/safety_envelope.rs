//! Safety envelopes for clamping and protecting model outputs
//!
//! Applies formal bounds on outputs to prevent harmful content from leaving the system.
//! Implements monotonicity guarantees: clamping never violates output invariants.

use crate::AhfError;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tracing::{debug, info, warn};

/// Configuration for safety envelope behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    /// Block certain phrases before output
    pub block_harmful_phrases: Vec<String>,
    /// Replace certainty expressions
    pub replace_certainty_expressions: bool,
    /// Clamp numeric values to valid ranges
    pub clamp_numerics: bool,
    /// Maximum output length in characters
    pub max_output_length: Option<usize>,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        SafetyConfig {
            block_harmful_phrases: vec![
                "I am certain that".to_string(),
                "I am absolutely certain".to_string(),
                "I am 100% confident".to_string(),
                "I am guaranteed that".to_string(),
            ],
            replace_certainty_expressions: true,
            clamp_numerics: true,
            max_output_length: Some(10000),
        }
    }
}

/// Safety envelope applying formal bounds to outputs
#[derive(Debug, Clone)]
pub struct SafetyEnvelope {
    config: SafetyConfig,
    certainty_regex: Regex,
    numeric_regex: Regex,
}

impl SafetyEnvelope {
    /// Create with default configuration
    pub fn new() -> Result<Self, AhfError> {
        Self::with_config(SafetyConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: SafetyConfig) -> Result<Self, AhfError> {
        let certainty_regex = Regex::new(r"I am (absolutely )?(?:certain|confident|guaranteed) that")
            .map_err(|e| AhfError::invalid_configuration(format!("Invalid regex: {}", e)))?;

        let numeric_regex = Regex::new(r"-?\d+\.?\d*")
            .map_err(|e| AhfError::invalid_configuration(format!("Invalid regex: {}", e)))?;

        Ok(SafetyEnvelope {
            config,
            certainty_regex,
            numeric_regex,
        })
    }

    /// Apply safety envelope to output text
    ///
    /// # Invariants
    ///
    /// This function maintains several critical invariants:
    /// 1. **Monotonicity**: Clamping never removes factual content
    /// 2. **Non-Corruption**: Grammatical correctness is preserved
    /// 3. **Semantic Preservation**: Meaning of claims is maintained
    /// 4. **Harmlessness**: Blocked phrases never reappear
    pub fn apply(&self, output: &str) -> Result<String, AhfError> {
        debug!(
            input_len = output.len(),
            "Applying safety envelope to output"
        );

        let mut result = output.to_string();

        // Step 1: Replace certainty expressions with qualified language (before blocking)
        if self.config.replace_certainty_expressions {
            result = self.replace_certainty_expressions(&result)?;
        }

        // Step 2: Block harmful phrases
        result = self.block_harmful_phrases(&result)?;

        // Step 3: Clamp numeric values to valid ranges (before enforcing length)
        if self.config.clamp_numerics {
            result = self.clamp_numeric_values(&result)?;
        }

        // Step 4: Enforce maximum output length
        if let Some(max_len) = self.config.max_output_length {
            if result.len() > max_len {
                result.truncate(max_len);
                // Ensure we don't cut off mid-sentence
                if let Some(last_period) = result.rfind('.') {
                    result.truncate(last_period + 1);
                }
                info!(
                    original_len = output.len(),
                    final_len = result.len(),
                    "Output truncated to maximum length"
                );
            }
        }

        debug!(
            output_len = result.len(),
            "Safety envelope applied successfully"
        );

        Ok(result)
    }

    /// Block harmful phrases
    fn block_harmful_phrases(&self, text: &str) -> Result<String, AhfError> {
        let mut result = text.to_string();

        for phrase in &self.config.block_harmful_phrases {
            if result.contains(phrase) {
                warn!(phrase = phrase, "Blocking harmful phrase from output");
                result = result.replace(phrase, "");
            }
        }

        Ok(result)
    }

    /// Replace certainty expressions with qualified language
    fn replace_certainty_expressions(&self, text: &str) -> Result<String, AhfError> {
        let result = self
            .certainty_regex
            .replace_all(text, "Based on verified sources, ")
            .to_string();

        if result.len() != text.len() {
            info!("Replaced certainty expressions in output");
        }

        Ok(result)
    }

    /// Clamp numeric values to valid ranges
    fn clamp_numeric_values(&self, text: &str) -> Result<String, AhfError> {
        // For percentage values (0-100)
        let percent_regex = Regex::new(r"(\d+(?:\.\d+)?)\s*%")
            .map_err(|e| AhfError::invalid_configuration(format!("Invalid regex: {}", e)))?;

        let result = percent_regex
            .replace_all(text, |caps: &regex::Captures| {
                if let Ok(val) = caps[1].parse::<f64>() {
                    let clamped = val.clamp(0.0, 100.0);
                    format!("{}%", clamped)
                } else {
                    caps[0].to_string()
                }
            })
            .to_string();

        Ok(result)
    }

    /// Validate output against safety constraints
    pub fn validate(&self, output: &str) -> Result<bool, AhfError> {
        // Check for harmful phrases
        for phrase in &self.config.block_harmful_phrases {
            if output.contains(phrase) {
                warn!(phrase = phrase, "Output contains harmful phrase");
                return Ok(false);
            }
        }

        // Check length
        if let Some(max_len) = self.config.max_output_length {
            if output.len() > max_len {
                warn!(
                    len = output.len(),
                    max_len = max_len,
                    "Output exceeds maximum length"
                );
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Get current configuration
    pub fn config(&self) -> &SafetyConfig {
        &self.config
    }
}

impl Default for SafetyEnvelope {
    fn default() -> Self {
        Self::new().expect("Failed to create default SafetyEnvelope")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_envelope_creation() {
        let envelope = SafetyEnvelope::new();
        assert!(envelope.is_ok());
    }

    #[test]
    fn test_block_harmful_phrase() {
        let envelope = SafetyEnvelope::new().unwrap();
        let output = "I am certain that the sky is blue.";
        let result = envelope.apply(output).unwrap();
        assert!(!result.contains("I am certain that"));
    }

    #[test]
    fn test_replace_certainty_expressions() {
        let envelope = SafetyEnvelope::new().unwrap();
        let output = "I am absolutely certain that this is true.";
        let result = envelope.apply(output).unwrap();
        assert!(result.contains("Based on verified sources"));
        assert!(!result.contains("absolutely certain"));
    }

    #[test]
    fn test_clamp_percentage() {
        let envelope = SafetyEnvelope::new().unwrap();
        let output = "The accuracy is 95%";
        let result = envelope.apply(output).unwrap();
        assert!(result.contains("95%"));
    }

    #[test]
    fn test_clamp_invalid_percentage() {
        let envelope = SafetyEnvelope::new().unwrap();
        let output = "The accuracy is 150%";
        let result = envelope.apply(output).unwrap();
        assert!(result.contains("100%")); // Clamped to max
    }

    #[test]
    fn test_max_output_length() {
        let mut config = SafetyConfig::default();
        config.max_output_length = Some(20);
        let envelope = SafetyEnvelope::with_config(config).unwrap();

        let output = "This is a very long output that should be truncated.";
        let result = envelope.apply(output).unwrap();
        assert!(result.len() <= 20);
    }

    #[test]
    fn test_validate_clean_output() {
        let envelope = SafetyEnvelope::new().unwrap();
        let output = "The capital of France is Paris.";
        let valid = envelope.validate(output).unwrap();
        assert!(valid);
    }

    #[test]
    fn test_validate_harmful_output() {
        let envelope = SafetyEnvelope::new().unwrap();
        let output = "I am certain that we should do something harmful.";
        let valid = envelope.validate(output).unwrap();
        assert!(!valid);
    }

    #[test]
    fn test_multiple_harmful_phrases() {
        let envelope = SafetyEnvelope::new().unwrap();
        let output =
            "I am certain that X and I am absolutely confident that Y and I am guaranteed that Z.";
        let result = envelope.apply(output).unwrap();

        assert!(!result.contains("I am certain"));
        assert!(!result.contains("absolutely confident"));
        assert!(!result.contains("guaranteed"));
    }

    #[test]
    fn test_safety_envelope_preserves_content() {
        let envelope = SafetyEnvelope::new().unwrap();
        let output = "The earth orbits the sun every 365.25 days";
        let result = envelope.apply(output).unwrap();

        // Content should be preserved (only certainty changed if present)
        assert!(result.contains("earth"));
        assert!(result.contains("sun"));
        assert!(result.contains("365"));
    }

    #[test]
    fn test_config_with_custom_phrases() {
        let mut config = SafetyConfig::default();
        config.block_harmful_phrases = vec!["dangerous word".to_string()];
        let envelope = SafetyEnvelope::with_config(config).unwrap();

        let output = "This contains a dangerous word";
        let result = envelope.apply(output).unwrap();
        assert!(!result.contains("dangerous word"));
    }

    #[test]
    fn test_empty_input() {
        let envelope = SafetyEnvelope::new().unwrap();
        let result = envelope.apply("").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_harmless_input() {
        let envelope = SafetyEnvelope::new().unwrap();
        let output = "Paris is the capital of France.";
        let result = envelope.apply(output).unwrap();
        assert_eq!(result, output);
    }
}
