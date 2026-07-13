//! Test plan parser for YAML and JSON formats
//!
//! Provides parsing and validation of test plan files in multiple formats.

use crate::schema::TestPlanYaml;
use serde_yaml;
use srwsts_core::{SrwstsError, SrwstsResult, TestPlan};
use std::path::Path;

/// Parser for SRWSTS test plans
pub struct TestPlanParser {
    strict_mode: bool,
}

impl TestPlanParser {
    /// Create a new test plan parser
    pub fn new() -> Self {
        Self {
            strict_mode: true,
        }
    }

    /// Set strict mode (validate all constraints)
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    /// Parse a test plan from YAML string
    pub fn parse_yaml(&self, yaml_str: &str) -> SrwstsResult<TestPlan> {
        let yaml_plan: TestPlanYaml = serde_yaml::from_str(yaml_str)
            .map_err(|e| SrwstsError::InvalidTestPlan {
                reason: format!("YAML parsing error: {}", e),
            })?;

        yaml_plan.to_core_plan()
    }

    /// Parse a test plan from a YAML file
    pub fn parse_file(&self, path: &Path) -> SrwstsResult<TestPlan> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| SrwstsError::FileReadError {
                path: path.to_string_lossy().to_string(),
                reason: e.to_string(),
            })?;

        self.parse_yaml(&content)
    }

    /// Parse a test plan from JSON string
    pub fn parse_json(&self, json_str: &str) -> SrwstsResult<TestPlan> {
        let yaml_plan: TestPlanYaml = serde_json::from_str(json_str)
            .map_err(|e| SrwstsError::InvalidTestPlan {
                reason: format!("JSON parsing error: {}", e),
            })?;

        yaml_plan.to_core_plan()
    }

    /// Parse a test plan from a JSON file
    pub fn parse_json_file(&self, path: &Path) -> SrwstsResult<TestPlan> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| SrwstsError::FileReadError {
                path: path.to_string_lossy().to_string(),
                reason: e.to_string(),
            })?;

        self.parse_json(&content)
    }

    /// Parse auto-detecting format from file (based on extension)
    pub fn parse_auto(&self, path: &Path) -> SrwstsResult<TestPlan> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("yaml");

        match ext {
            "json" => self.parse_json_file(path),
            "yaml" | "yml" => self.parse_file(path),
            _ => Err(SrwstsError::InvalidConfiguration {
                reason: format!("unsupported file extension: {}", ext),
            }),
        }
    }

    /// Validate that a YAML structure is parseable without converting
    pub fn validate_yaml(&self, yaml_str: &str) -> SrwstsResult<()> {
        let _: TestPlanYaml = serde_yaml::from_str(yaml_str)
            .map_err(|e| SrwstsError::InvalidTestPlan {
                reason: format!("YAML validation error: {}", e),
            })?;
        Ok(())
    }
}

impl Default for TestPlanParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_yaml() {
        let yaml = r#"
version: "1.0"
metadata:
  id: "test1"
  name: "Test Plan"
  description: "Test"
resource_limits:
  max_cpu_percent: 50
workloads:
  - id: "w1"
    type: "cpu_stress"
    concurrency: 4
    duration_secs: 300
"#;
        let parser = TestPlanParser::new();
        let plan = parser.parse_yaml(yaml);
        assert!(plan.is_ok());
        let plan = plan.unwrap();
        assert_eq!(plan.id, "test1");
        assert_eq!(plan.workloads.len(), 1);
    }

    #[test]
    fn test_parse_with_faults() {
        let yaml = r#"
version: "1.0"
metadata:
  id: "test-with-faults"
  name: "Test With Faults"
  description: "Test"
workloads:
  - id: "w1"
    type: "cpu_stress"
    concurrency: 4
    duration_secs: 300
faults:
  - id: "f1"
    type: "cpu_stress"
    inject_at_secs: 60
    duration_secs: 120
"#;
        let parser = TestPlanParser::new();
        let plan = parser.parse_yaml(yaml);
        assert!(plan.is_ok());
        let plan = plan.unwrap();
        assert_eq!(plan.faults.len(), 1);
    }

    #[test]
    fn test_parse_invalid_yaml() {
        let parser = TestPlanParser::new();
        let result = parser.parse_yaml("invalid: [yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_yaml() {
        let yaml = r#"
version: "1.0"
metadata:
  id: "test1"
  name: "Test"
  description: "Test"
workloads:
  - id: "w1"
    type: "cpu_stress"
    concurrency: 4
    duration_secs: 300
"#;
        let parser = TestPlanParser::new();
        assert!(parser.validate_yaml(yaml).is_ok());
    }

    #[test]
    fn test_parse_with_metadata_tags() {
        let yaml = r#"
version: "1.0"
metadata:
  id: "test-tagged"
  name: "Tagged Test"
  description: "Test with tags"
  tags:
    - kernel
    - stress
    - resilience
workloads:
  - id: "w1"
    type: "cpu_stress"
    concurrency: 4
    duration_secs: 300
"#;
        let parser = TestPlanParser::new();
        let plan = parser.parse_yaml(yaml).unwrap();
        assert!(plan.metadata.contains_key("tag:kernel"));
        assert!(plan.metadata.contains_key("tag:stress"));
    }
}
