//! Equivalence reporting and root cause analysis

use crate::{
    ArchitectureTestResults, EquivalenceReport,
    TraceComparator,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Detailed equivalence report with root cause analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedEquivalenceReport {
    /// Base report
    pub report: EquivalenceReport,
    /// Test results
    pub test_results: ArchitectureTestResults,
    /// Per-architecture performance summary
    pub performance_summary: HashMap<String, PerformanceSummary>,
    /// Divergence analysis
    pub divergence_analysis: Option<DivergenceAnalysis>,
    /// Recommendations
    pub recommendations: Vec<String>,
}

impl DetailedEquivalenceReport {
    /// Create from test results and report
    pub fn new(
        report: EquivalenceReport,
        test_results: ArchitectureTestResults,
    ) -> Self {
        let mut perf_summary = HashMap::new();

        for result in &test_results.results {
            perf_summary.insert(
                result.architecture.to_string(),
                PerformanceSummary {
                    architecture: result.architecture.to_string(),
                    execution_time_ns: result.exec_time_ns,
                    instruction_count: result.trace.instruction_count,
                    branch_count: result.trace.branch_count,
                    l1_hit_ratio: result.memory_trace.l1_hit_ratio(),
                    l2_hit_ratio: result.memory_trace.l2_hit_ratio(),
                },
            );
        }

        Self {
            report,
            test_results,
            performance_summary: perf_summary,
            divergence_analysis: None,
            recommendations: Vec::new(),
        }
    }

    /// Perform root cause analysis
    pub fn analyze_root_causes(&mut self) -> Option<()> {
        if self.test_results.results.is_empty() {
            return None;
        }

        let reference = &self.test_results.results[0];

        for result in &self.test_results.results[1..] {
            if let Some(divergence_point) = TraceComparator::find_divergence(&reference.trace, &result.trace) {
                self.divergence_analysis = Some(DivergenceAnalysis {
                    divergence_point,
                    affected_architectures: vec![
                        reference.architecture.to_string(),
                        result.architecture.to_string(),
                    ],
                    root_cause: "Execution trace divergence detected".to_string(),
                    remediation: Some("Check architecture-specific code paths".to_string()),
                });
                return Some(());
            }
        }

        None
    }

    /// Add recommendations
    pub fn add_recommendation(&mut self, recommendation: String) {
        self.recommendations.push(recommendation);
    }

    /// Generate HTML report
    pub fn to_html(&self) -> String {
        let mut html = String::new();

        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str("<title>Equivalence Report</title>\n");
        html.push_str("<style>\n");
        html.push_str("body { font-family: monospace; }\n");
        html.push_str(".green { color: green; font-weight: bold; }\n");
        html.push_str(".yellow { color: orange; font-weight: bold; }\n");
        html.push_str(".red { color: red; font-weight: bold; }\n");
        html.push_str("table { border-collapse: collapse; }\n");
        html.push_str("th, td { border: 1px solid black; padding: 8px; text-align: left; }\n");
        html.push_str("</style>\n</head>\n<body>\n");

        html.push_str(&format!("<h1>Equivalence Report: {}</h1>\n", self.report.test_name));

        // Status
        let status_class = match self.report.status {
            crate::EquivalenceStatus::Green => "green",
            crate::EquivalenceStatus::Yellow => "yellow",
            crate::EquivalenceStatus::Red => "red",
            crate::EquivalenceStatus::Unknown => "unknown",
        };
        html.push_str(&format!("<p>Status: <span class=\"{}\">{}</span></p>\n",
            status_class, self.report.status));

        // Validations
        html.push_str("<h2>Validation Results</h2>\n");
        html.push_str("<table>\n");
        html.push_str("<tr><th>Validator</th><th>Status</th><th>Message</th></tr>\n");

        for validation in &self.report.validations {
            let val_class = match validation.status {
                crate::EquivalenceStatus::Green => "green",
                crate::EquivalenceStatus::Yellow => "yellow",
                crate::EquivalenceStatus::Red => "red",
                crate::EquivalenceStatus::Unknown => "unknown",
            };
            html.push_str(&format!(
                "<tr><td>{}</td><td class=\"{}\">{}</td><td>{}</td></tr>\n",
                validation.validator_name, val_class, validation.status, validation.message
            ));
        }

        html.push_str("</table>\n");

        // Performance Summary
        html.push_str("<h2>Performance Summary</h2>\n");
        html.push_str("<table>\n");
        html.push_str("<tr><th>Architecture</th><th>Exec Time (ns)</th><th>L1 Hit Ratio</th><th>L2 Hit Ratio</th></tr>\n");

        for (arch, summary) in &self.performance_summary {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{:.2}%</td><td>{:.2}%</td></tr>\n",
                arch, summary.execution_time_ns,
                summary.l1_hit_ratio * 100.0,
                summary.l2_hit_ratio * 100.0
            ));
        }

        html.push_str("</table>\n");

        html.push_str("</body>\n</html>");

        html
    }

    /// Generate JSON report
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Performance summary per architecture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    /// Architecture
    pub architecture: String,
    /// Execution time in nanoseconds
    pub execution_time_ns: u64,
    /// Total instruction count
    pub instruction_count: u64,
    /// Total branch count
    pub branch_count: u64,
    /// L1 cache hit ratio (0.0 to 1.0)
    pub l1_hit_ratio: f64,
    /// L2 cache hit ratio (0.0 to 1.0)
    pub l2_hit_ratio: f64,
}

/// Divergence analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergenceAnalysis {
    /// Instruction count where divergence occurred
    pub divergence_point: u64,
    /// Affected architectures
    pub affected_architectures: Vec<String>,
    /// Root cause description
    pub root_cause: String,
    /// Suggested remediation
    pub remediation: Option<String>,
}

/// Cross-architecture comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossArchitectureComparison {
    /// Architectures being compared
    pub architectures: Vec<String>,
    /// Execution time comparison (nanoseconds per architecture)
    pub execution_times: HashMap<String, u64>,
    /// Memory access comparison
    pub memory_access: HashMap<String, MemoryComparison>,
    /// Atomic operation comparison
    pub atomic_operations: HashMap<String, u64>,
}

impl CrossArchitectureComparison {
    /// Create a new comparison
    pub fn new(architectures: Vec<String>) -> Self {
        Self {
            architectures,
            execution_times: HashMap::new(),
            memory_access: HashMap::new(),
            atomic_operations: HashMap::new(),
        }
    }

    /// Add execution time
    pub fn add_execution_time(&mut self, arch: String, time_ns: u64) {
        self.execution_times.insert(arch, time_ns);
    }

    /// Add memory access comparison
    pub fn add_memory_comparison(&mut self, arch: String, comparison: MemoryComparison) {
        self.memory_access.insert(arch, comparison);
    }

    /// Calculate performance delta between architectures
    pub fn performance_delta(&self, arch1: &str, arch2: &str) -> Option<f64> {
        let time1 = self.execution_times.get(arch1)?;
        let time2 = self.execution_times.get(arch2)?;

        if *time2 == 0 {
            return None;
        }

        Some((*time1 as f64 - *time2 as f64) / *time2 as f64 * 100.0)
    }

    /// Check if all architectures are within tolerance
    pub fn within_tolerance(&self, tolerance_percent: f64) -> bool {
        for i in 0..self.architectures.len() {
            for j in (i + 1)..self.architectures.len() {
                if let Some(delta) = self.performance_delta(
                    &self.architectures[i],
                    &self.architectures[j],
                ) {
                    if delta.abs() > tolerance_percent {
                        return false;
                    }
                }
            }
        }
        true
    }
}

/// Memory access comparison
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MemoryComparison {
    /// L1 hit ratio
    pub l1_hits: u64,
    /// L1 misses
    pub l1_misses: u64,
    /// L2 hit ratio
    pub l2_hits: u64,
    /// L2 misses
    pub l2_misses: u64,
    /// Total load count
    pub load_count: u64,
    /// Total store count
    pub store_count: u64,
}

impl MemoryComparison {
    /// Get L1 hit ratio
    pub fn l1_hit_ratio(&self) -> f64 {
        if self.l1_hits + self.l1_misses == 0 {
            return 0.0;
        }
        self.l1_hits as f64 / (self.l1_hits + self.l1_misses) as f64
    }

    /// Get L2 hit ratio
    pub fn l2_hit_ratio(&self) -> f64 {
        if self.l2_hits + self.l2_misses == 0 {
            return 0.0;
        }
        self.l2_hits as f64 / (self.l2_hits + self.l2_misses) as f64
    }
}

/// Equivalence report formatter
pub struct EquivalenceReportFormatter;

impl EquivalenceReportFormatter {
    /// Format report as plain text table
    pub fn format_table(report: &DetailedEquivalenceReport) -> String {
        let mut output = String::new();

        output.push_str("╔════════════════════════════════════════════════════════════╗\n");
        output.push_str("║           EQUIVALENCE VALIDATION REPORT                    ║\n");
        output.push_str("╚════════════════════════════════════════════════════════════╝\n\n");

        output.push_str(&format!("Test: {}\n", report.report.test_name));
        output.push_str(&format!("Status: {}\n", report.report.status));
        output.push_str(&format!("Run ID: {}\n", report.test_results.run_id));
        output.push_str(&format!("Architectures: {}\n\n", report.test_results.results.len()));

        // Performance table
        output.push_str("┌─────────────────────┬──────────────────┬─────────────┬─────────────┐\n");
        output.push_str("│ Architecture        │ Exec Time (ns)   │ L1 Hit Ratio│ L2 Hit Ratio│\n");
        output.push_str("├─────────────────────┼──────────────────┼─────────────┼─────────────┤\n");

        for (arch, summary) in &report.performance_summary {
            output.push_str(&format!(
                "│ {:<19} │ {:<16} │ {:<11.1}% │ {:<11.1}% │\n",
                arch,
                summary.execution_time_ns,
                summary.l1_hit_ratio * 100.0,
                summary.l2_hit_ratio * 100.0
            ));
        }

        output.push_str("└─────────────────────┴──────────────────┴─────────────┴─────────────┘\n\n");

        // Validations
        output.push_str("Validations:\n");
        for validation in &report.report.validations {
            let symbol = match validation.status {
                crate::EquivalenceStatus::Green => "✓",
                crate::EquivalenceStatus::Yellow => "⚠",
                crate::EquivalenceStatus::Red => "✗",
                crate::EquivalenceStatus::Unknown => "?",
            };
            output.push_str(&format!("  {} {}: {}\n", symbol, validation.validator_name, validation.message));
        }

        output
    }

    /// Format report as markdown
    pub fn format_markdown(report: &DetailedEquivalenceReport) -> String {
        let mut output = String::new();

        output.push_str(&format!("# Equivalence Report: {}\n\n", report.report.test_name));
        output.push_str(&format!("**Status**: {}\n\n", report.report.status));

        output.push_str("## Performance Summary\n\n");
        output.push_str("| Architecture | Exec Time (ns) | L1 Hit Ratio | L2 Hit Ratio |\n");
        output.push_str("|---|---|---|---|\n");

        for (arch, summary) in &report.performance_summary {
            output.push_str(&format!(
                "| {} | {} | {:.2}% | {:.2}% |\n",
                arch,
                summary.execution_time_ns,
                summary.l1_hit_ratio * 100.0,
                summary.l2_hit_ratio * 100.0
            ));
        }

        output.push_str("\n## Validation Results\n\n");

        for validation in &report.report.validations {
            let status_icon = match validation.status {
                crate::EquivalenceStatus::Green => "✓",
                crate::EquivalenceStatus::Yellow => "⚠",
                crate::EquivalenceStatus::Red => "✗",
                crate::EquivalenceStatus::Unknown => "?",
            };
            output.push_str(&format!("- **{} {}**: {}\n", status_icon, validation.validator_name, validation.message));
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArchitectureTarget, ArchVariant, ArchitectureTestResults};

    #[test]
    fn test_performance_summary() {
        let summary = PerformanceSummary {
            architecture: "x86_64".to_string(),
            execution_time_ns: 1000,
            instruction_count: 100,
            branch_count: 10,
            l1_hit_ratio: 0.9,
            l2_hit_ratio: 0.8,
        };

        assert_eq!(summary.l1_hit_ratio, 0.9);
    }

    #[test]
    fn test_comparison_performance_delta() {
        let mut comparison = CrossArchitectureComparison::new(vec![
            "x86_64".to_string(),
            "armv8".to_string(),
        ]);

        comparison.add_execution_time("x86_64".to_string(), 1000);
        comparison.add_execution_time("armv8".to_string(), 1100);

        let delta = comparison.performance_delta("armv8", "x86_64");
        assert!(delta.is_some());
        assert!(delta.unwrap() > 0.0);
    }

    #[test]
    fn test_memory_comparison() {
        let comparison = MemoryComparison {
            l1_hits: 900,
            l1_misses: 100,
            l2_hits: 80,
            l2_misses: 20,
            load_count: 1000,
            store_count: 500,
        };

        assert!((comparison.l1_hit_ratio() - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_within_tolerance() {
        let mut comparison = CrossArchitectureComparison::new(vec![
            "x86_64".to_string(),
            "armv8".to_string(),
        ]);

        comparison.add_execution_time("x86_64".to_string(), 1000);
        comparison.add_execution_time("armv8".to_string(), 1050);

        assert!(comparison.within_tolerance(10.0));
        assert!(!comparison.within_tolerance(3.0));
    }
}
