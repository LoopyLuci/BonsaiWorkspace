use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Coverage gate configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageGate {
    pub name: String,
    pub target_coverage_percent: f64,
    pub max_regression_percent: f64,
    pub crates: Vec<String>, // Empty = applies to all
    pub enforce: bool,       // If false, warn only
}

/// Gate check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateCheckResult {
    pub gate_name: String,
    pub passed: bool,
    pub current_coverage: f64,
    pub target_coverage: f64,
    pub regression: f64,
    pub failed_crates: Vec<String>,
    pub message: String,
}

/// Enforces coverage gates
pub struct CoverageEnforcer {
    gates: Vec<CoverageGate>,
    baseline_coverage: HashMap<String, f64>,
}

impl CoverageEnforcer {
    pub fn new() -> Self {
        Self {
            gates: Vec::new(),
            baseline_coverage: HashMap::new(),
        }
    }

    /// Add a coverage gate
    pub fn add_gate(&mut self, gate: CoverageGate) {
        self.gates.push(gate);
    }

    /// Set baseline coverage for regression detection
    pub fn set_baseline(&mut self, crate_name: &str, coverage: f64) {
        self.baseline_coverage.insert(crate_name.to_string(), coverage);
    }

    /// Check all gates
    pub fn check_all_gates(
        &self,
        crate_coverage: &HashMap<String, f64>,
    ) -> Vec<GateCheckResult> {
        let mut results = Vec::new();

        for gate in &self.gates {
            let result = self.check_gate(gate, crate_coverage);
            results.push(result);
        }

        results
    }

    /// Check single gate
    pub fn check_gate(
        &self,
        gate: &CoverageGate,
        crate_coverage: &HashMap<String, f64>,
    ) -> GateCheckResult {
        let mut failed_crates = Vec::new();
        let mut total_coverage = 0.0;
        let mut crate_count = 0;

        // Determine which crates to check
        let crates_to_check: Vec<&String> = if gate.crates.is_empty() {
            crate_coverage.keys().collect()
        } else {
            gate.crates.iter().collect()
        };

        // Check coverage targets
        for crate_name in &crates_to_check {
            if let Some(&coverage) = crate_coverage.get(*crate_name) {
                if coverage < gate.target_coverage_percent {
                    failed_crates.push((*crate_name).clone());
                }

                // Check regression
                if let Some(&baseline) = self.baseline_coverage.get(*crate_name) {
                    let regression = baseline - coverage;
                    if regression > gate.max_regression_percent {
                        if !failed_crates.contains(crate_name) {
                            failed_crates.push((*crate_name).clone());
                        }
                    }
                }

                total_coverage += coverage;
                crate_count += 1;
            }
        }

        let avg_coverage = if crate_count > 0 {
            total_coverage / crate_count as f64
        } else {
            0.0
        };

        let passed = failed_crates.is_empty();

        let message = if passed {
            format!(
                "✓ Gate '{}': Passed ({}% coverage)",
                gate.name, avg_coverage as i32
            )
        } else {
            format!(
                "✗ Gate '{}': Failed ({} crates below target)",
                gate.name,
                failed_crates.len()
            )
        };

        GateCheckResult {
            gate_name: gate.name.clone(),
            passed,
            current_coverage: avg_coverage,
            target_coverage: gate.target_coverage_percent,
            regression: 0.0,
            failed_crates,
            message,
        }
    }

    /// Check if any critical gate failed
    pub fn has_critical_failures(results: &[GateCheckResult]) -> bool {
        results.iter().any(|r| !r.passed)
    }

    /// Get enforcement summary
    pub fn get_summary(results: &[GateCheckResult]) -> EnforcementSummary {
        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = total - passed;

        EnforcementSummary {
            total_gates: total,
            passed_gates: passed,
            failed_gates: failed,
            all_passed: failed == 0,
        }
    }
}

impl Default for CoverageEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of gate enforcement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementSummary {
    pub total_gates: usize,
    pub passed_gates: usize,
    pub failed_gates: usize,
    pub all_passed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coverage_gate() {
        let gate = CoverageGate {
            name: "minimum_coverage".to_string(),
            target_coverage_percent: 80.0,
            max_regression_percent: 5.0,
            crates: vec![],
            enforce: true,
        };

        let mut enforcer = CoverageEnforcer::new();
        enforcer.add_gate(gate);

        let mut coverage = HashMap::new();
        coverage.insert("test_crate".to_string(), 85.0);

        let results = enforcer.check_all_gates(&coverage);
        assert_eq!(results.len(), 1);
        assert!(results[0].passed);
    }

    #[test]
    fn test_gate_failure() {
        let gate = CoverageGate {
            name: "minimum_coverage".to_string(),
            target_coverage_percent: 80.0,
            max_regression_percent: 5.0,
            crates: vec![],
            enforce: true,
        };

        let mut enforcer = CoverageEnforcer::new();
        enforcer.add_gate(gate);

        let mut coverage = HashMap::new();
        coverage.insert("test_crate".to_string(), 70.0);

        let results = enforcer.check_all_gates(&coverage);
        assert!(!results[0].passed);
        assert!(!results[0].failed_crates.is_empty());
    }
}
