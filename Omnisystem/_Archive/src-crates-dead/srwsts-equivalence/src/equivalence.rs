//! Core equivalence validation and reporting

use crate::{ArchitectureTestResults, EquivalenceConfig, EquivalenceResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Status of equivalence validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EquivalenceStatus {
    /// All checks passed
    Green,
    /// Checks passed but with caveats
    Yellow,
    /// Checks failed
    Red,
    /// Unknown status
    Unknown,
}

impl fmt::Display for EquivalenceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Green => write!(f, "GREEN"),
            Self::Yellow => write!(f, "YELLOW"),
            Self::Red => write!(f, "RED"),
            Self::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Result of a single validation check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Name of the validator
    pub validator_name: String,
    /// Status of the validation
    pub status: EquivalenceStatus,
    /// Description of the result
    pub message: String,
    /// Details for the result
    pub details: HashMap<String, String>,
    /// Whether this check is critical (failure blocks overall pass)
    pub is_critical: bool,
}

impl ValidationResult {
    /// Create a passing validation result
    pub fn pass(name: String) -> Self {
        Self {
            validator_name: name,
            status: EquivalenceStatus::Green,
            message: "Validation passed".to_string(),
            details: HashMap::new(),
            is_critical: true,
        }
    }

    /// Create a warning validation result
    pub fn warn(name: String, message: String) -> Self {
        Self {
            validator_name: name,
            status: EquivalenceStatus::Yellow,
            message,
            details: HashMap::new(),
            is_critical: false,
        }
    }

    /// Create a failing validation result
    pub fn fail(name: String, message: String) -> Self {
        Self {
            validator_name: name,
            status: EquivalenceStatus::Red,
            message,
            details: HashMap::new(),
            is_critical: true,
        }
    }

    /// Add a detail to this result
    pub fn with_detail(mut self, key: String, value: String) -> Self {
        self.details.insert(key, value);
        self
    }
}

/// Complete equivalence report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquivalenceReport {
    /// Name of the test
    pub test_name: String,
    /// Overall status
    pub status: EquivalenceStatus,
    /// Individual validation results
    pub validations: Vec<ValidationResult>,
    /// Root cause analysis
    pub root_cause_analysis: Option<RootCauseAnalysis>,
    /// Summary message
    pub summary: String,
}

impl EquivalenceReport {
    /// Create a new report
    pub fn new(test_name: impl Into<String>, validations: Vec<ValidationResult>) -> Self {
        let test_name = test_name.into();

        // Determine overall status
        let status = if validations.iter().any(|v| v.is_critical && v.status == EquivalenceStatus::Red) {
            EquivalenceStatus::Red
        } else if validations.iter().any(|v| v.status == EquivalenceStatus::Yellow) {
            EquivalenceStatus::Yellow
        } else {
            EquivalenceStatus::Green
        };

        let summary = format!("Test '{}' completed with status: {}", test_name, status);

        Self {
            test_name,
            status,
            validations,
            root_cause_analysis: None,
            summary,
        }
    }

    /// Add root cause analysis
    pub fn with_rca(mut self, rca: RootCauseAnalysis) -> Self {
        self.root_cause_analysis = Some(rca);
        self
    }

    /// Get all failed validations
    pub fn failed_validations(&self) -> Vec<&ValidationResult> {
        self.validations
            .iter()
            .filter(|v| v.status == EquivalenceStatus::Red)
            .collect()
    }

    /// Get all warning validations
    pub fn warning_validations(&self) -> Vec<&ValidationResult> {
        self.validations
            .iter()
            .filter(|v| v.status == EquivalenceStatus::Yellow)
            .collect()
    }

    /// Check if equivalence passed
    pub fn passed(&self) -> bool {
        self.status != EquivalenceStatus::Red
    }
}

impl fmt::Display for EquivalenceReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "============================================")?;
        writeln!(f, "EQUIVALENCE VALIDATION REPORT")?;
        writeln!(f, "============================================")?;
        writeln!(f)?;
        writeln!(f, "Test: {}", self.test_name)?;
        writeln!(f, "Status: {}", self.status)?;
        writeln!(f)?;

        writeln!(f, "--- VALIDATION RESULTS ---")?;
        for validation in &self.validations {
            let symbol = match validation.status {
                EquivalenceStatus::Green => "✓",
                EquivalenceStatus::Yellow => "⚠",
                EquivalenceStatus::Red => "✗",
                EquivalenceStatus::Unknown => "?",
            };
            writeln!(f, "{} {}: {}", symbol, validation.validator_name, validation.message)?;
            for (key, value) in &validation.details {
                writeln!(f, "    {}: {}", key, value)?;
            }
        }

        writeln!(f)?;

        if let Some(rca) = &self.root_cause_analysis {
            writeln!(f, "--- ROOT CAUSE ANALYSIS ---")?;
            writeln!(f, "Divergence Point: {}", rca.divergence_point)?;
            writeln!(f, "Root Cause: {}", rca.root_cause)?;
            writeln!(f, "Affected Architectures: {}", rca.affected_architectures.join(", "))?;
            writeln!(f)?;
        }

        writeln!(f, "Summary: {}", self.summary)?;
        writeln!(f, "============================================")?;

        Ok(())
    }
}

/// Root cause analysis for divergences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootCauseAnalysis {
    /// Where the divergence occurred
    pub divergence_point: String,
    /// Root cause description
    pub root_cause: String,
    /// Architectures affected
    pub affected_architectures: Vec<String>,
    /// Suggested remediation
    pub remediation: Option<String>,
}

/// Output validator
#[derive(Default)]
pub struct OutputValidator;

#[async_trait]
impl crate::EquivalenceValidator for OutputValidator {
    async fn validate(
        &self,
        results: &ArchitectureTestResults,
        _config: &EquivalenceConfig,
    ) -> EquivalenceResult<ValidationResult> {
        if results.results.is_empty() {
            return Ok(ValidationResult::fail(
                self.name().to_string(),
                "No results to validate".to_string(),
            ));
        }

        // Check if all outputs match
        if !results.all_outputs_match() {
            let first_arch = &results.results[0].architecture;
            let first_hash = &results.results[0].output_hash;

            let mismatches: Vec<String> = results
                .results
                .iter()
                .skip(1)
                .filter(|r| &r.output_hash != first_hash)
                .map(|r| format!("{} (hash: {})", r.architecture, r.output_hash))
                .collect();

            return Ok(ValidationResult::fail(
                self.name().to_string(),
                format!(
                    "Output mismatch detected. Reference: {} (hash: {}). Mismatches: {}",
                    first_arch, first_hash,
                    mismatches.join(", ")
                ),
            ));
        }

        let result = ValidationResult::pass(self.name().to_string())
            .with_detail(
                "architectures_compared".to_string(),
                results.results.len().to_string(),
            )
            .with_detail(
                "output_hash".to_string(),
                results.results[0].output_hash.clone(),
            )
            .with_detail(
                "output_size_bytes".to_string(),
                results.results[0].output.len().to_string(),
            );

        Ok(result)
    }

    fn name(&self) -> &str {
        "Output Validator"
    }
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equivalence_status_display() {
        assert_eq!(EquivalenceStatus::Green.to_string(), "GREEN");
        assert_eq!(EquivalenceStatus::Yellow.to_string(), "YELLOW");
        assert_eq!(EquivalenceStatus::Red.to_string(), "RED");
    }

    #[test]
    fn test_validation_result_creation() {
        let result = ValidationResult::pass("test".to_string());
        assert_eq!(result.status, EquivalenceStatus::Green);
        assert!(result.is_critical);
    }

    #[test]
    fn test_report_status_determination() {
        let validations = vec![
            ValidationResult::pass("test1".to_string()),
            ValidationResult::warn("test2".to_string(), "warning".to_string()),
        ];

        let report = EquivalenceReport::new("test", validations);
        assert_eq!(report.status, EquivalenceStatus::Yellow);
    }

    #[test]
    fn test_report_failed_validations() {
        let validations = vec![
            ValidationResult::pass("test1".to_string()),
            ValidationResult::fail("test2".to_string(), "failed".to_string()),
            ValidationResult::fail("test3".to_string(), "failed".to_string()),
        ];

        let report = EquivalenceReport::new("test", validations);
        assert_eq!(report.failed_validations().len(), 2);
    }

    #[test]
    fn test_report_passed() {
        let validations = vec![
            ValidationResult::pass("test1".to_string()),
            ValidationResult::warn("test2".to_string(), "warning".to_string()),
        ];

        let report = EquivalenceReport::new("test", validations);
        assert!(report.passed());
    }

    #[test]
    fn test_report_failed() {
        let validations = vec![
            ValidationResult::fail("test1".to_string(), "failed".to_string()),
        ];

        let report = EquivalenceReport::new("test", validations);
        assert!(!report.passed());
    }
}
