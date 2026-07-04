/// Test Specification Format and Loader
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A single test case with deterministic inputs and expected output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    /// Name of the test case
    pub name: String,
    /// Input (as JSON string or raw value)
    pub input: String,
    /// Expected output (as JSON string or raw value)
    pub expected: String,
    /// Seed for deterministic generation (optional)
    pub seed: Option<u64>,
}

/// Specification of a deterministic test suite for UBVM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSpec {
    /// Name of this test suite
    pub name: String,
    /// Description of what this suite validates
    pub description: String,
    /// Bonsai subsystems this tests (e.g., ["language", "networking"])
    pub subsystems: Vec<String>,
    /// Reference language (e.g., "rust") that produces the oracle
    pub reference_lang: String,
    /// Canonical implementation (inline code, path, or language-specific source)
    pub canonical_source: String,
    /// Languages to validate against (must include reference_lang)
    pub languages: Vec<String>,
    /// Test cases (if empty, one default case is generated)
    pub test_cases: Vec<TestCase>,
    /// Runner templates: language -> command with {src}, {input}, {seed} substitution
    pub runners: HashMap<String, String>,
    /// Optional: fidelity tolerance (0.0..=1.0, default 0.99)
    pub fidelity_threshold: Option<f64>,
    /// Optional: timeout per test in seconds
    pub timeout_secs: Option<u64>,
}

impl Default for TestSpec {
    fn default() -> Self {
        Self {
            name: "Unnamed".to_string(),
            description: String::new(),
            subsystems: vec!["language".to_string()],
            reference_lang: "rust".to_string(),
            canonical_source: String::new(),
            languages: vec!["rust".to_string()],
            test_cases: vec![],
            runners: HashMap::new(),
            fidelity_threshold: Some(0.99),
            timeout_secs: Some(30),
        }
    }
}

impl TestSpec {
    /// Load a TestSpec from a TOML file
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let spec: TestSpec = toml::from_str(&content)?;
        if spec.canonical_source.is_empty() {
            anyhow::bail!("canonical_source must not be empty");
        }
        Ok(spec)
    }

    /// Load from TOML string
    pub fn from_toml(toml_str: &str) -> anyhow::Result<Self> {
        let spec: TestSpec = toml::from_str(toml_str)?;
        Ok(spec)
    }

    /// Get the fidelity threshold (default 0.99 if not specified)
    pub fn fidelity_threshold(&self) -> f64 {
        self.fidelity_threshold.unwrap_or(0.99)
    }

    /// Get timeout in seconds (default 30 if not specified)
    pub fn timeout_secs(&self) -> u64 {
        self.timeout_secs.unwrap_or(30)
    }

    /// Validate that the spec is well-formed
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.name.is_empty() {
            anyhow::bail!("spec.name must not be empty");
        }
        if self.canonical_source.is_empty() {
            anyhow::bail!("spec.canonical_source must not be empty");
        }
        if self.languages.is_empty() {
            anyhow::bail!("spec.languages must not be empty");
        }
        if !self.languages.contains(&self.reference_lang) {
            anyhow::bail!(
                "reference_lang '{}' must be in languages list",
                self.reference_lang
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_default() {
        let spec = TestSpec::default();
        assert_eq!(spec.fidelity_threshold(), 0.99);
        assert_eq!(spec.timeout_secs(), 30);
    }

    #[test]
    fn test_spec_validate() {
        let mut spec = TestSpec::default();
        spec.canonical_source = "fn add(a, b) { a + b }".to_string();
        assert!(spec.validate().is_ok());
    }
}
