use crate::{
    collector::CoverageCollector, enforcer::CoverageEnforcer, reporting::CoverageReporter,
    history::CoverageHistory,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// CI/CD coverage integration
pub struct CICoverageIntegration {
    collector: CoverageCollector,
    enforcer: CoverageEnforcer,
    reporter: CoverageReporter,
    history: CoverageHistory,
}

/// Coverage check result for CI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CICoverageCheckResult {
    pub passed: bool,
    pub coverage_percent: f64,
    pub target_coverage: f64,
    pub failed_crates: Vec<String>,
    pub report_markdown: String,
    pub report_json: String,
}

impl CICoverageIntegration {
    pub fn new(target_coverage: f64) -> Self {
        Self {
            collector: CoverageCollector::new(),
            enforcer: CoverageEnforcer::new(),
            reporter: CoverageReporter::new(target_coverage),
            history: CoverageHistory::default(),
        }
    }

    /// Record coverage from tarpaulin output
    pub fn record_coverage(&self, results: Vec<CoverageData>) {
        for result in results {
            self.collector.record_crate(
                &result.crate_name,
                result.lines_covered,
                result.lines_total,
                result.branch_coverage,
                result.files,
            );

            // Record in history
            self.history.record(
                &result.crate_name,
                result.coverage_percent,
                result.lines_covered,
                result.lines_total,
            );
        }
    }

    /// Perform full CI coverage check
    pub fn check_coverage(&self) -> CICoverageCheckResult {
        let results = self.collector.get_all_results();
        let agg = self.collector.get_aggregate_coverage();

        // Create crate coverage map
        let mut crate_coverage = HashMap::new();
        for result in &results {
            crate_coverage.insert(result.crate_name.clone(), result.coverage_percent);
        }

        // Check gates
        let gate_results = self.enforcer.check_all_gates(&crate_coverage);
        let all_passed = gate_results.iter().all(|r| r.passed);

        // Generate reports
        let report = self.reporter.generate(&results, None);
        let report_markdown = self.reporter.export_markdown(&report);
        let report_json = self.reporter.export_json(&report).unwrap_or_default();

        // Collect failed crates
        let failed_crates: Vec<String> = gate_results
            .iter()
            .flat_map(|r| r.failed_crates.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        CICoverageCheckResult {
            passed: all_passed,
            coverage_percent: agg.overall_coverage_percent,
            target_coverage: self.reporter.target_coverage,
            failed_crates,
            report_markdown,
            report_json,
        }
    }

    /// Get coverage collector
    pub fn collector(&self) -> &CoverageCollector {
        &self.collector
    }

    /// Get coverage enforcer
    pub fn enforcer(&self) -> &CoverageEnforcer {
        &self.enforcer
    }

    /// Get coverage reporter
    pub fn reporter(&self) -> &CoverageReporter {
        &self.reporter
    }

    /// Get coverage history
    pub fn history(&self) -> &CoverageHistory {
        &self.history
    }
}

/// Coverage data from tarpaulin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageData {
    pub crate_name: String,
    pub lines_covered: usize,
    pub lines_total: usize,
    pub branch_coverage: f64,
    pub coverage_percent: f64,
    pub files: Vec<crate::collector::FileCoverage>,
}

/// Generate coverage badge URL
pub fn generate_badge_url(coverage_percent: f64) -> String {
    let color = match coverage_percent {
        c if c >= 80.0 => "brightgreen",
        c if c >= 60.0 => "yellow",
        _ => "red",
    };

    format!(
        "https://img.shields.io/badge/coverage-{:.0}%25-{}",
        coverage_percent, color
    )
}

/// Parse tarpaulin XML output (simplified)
pub fn parse_tarpaulin_output(xml_content: &str) -> Result<Vec<CoverageData>, String> {
    // Simplified parser - in production would use proper XML parsing
    // This is a placeholder for the actual tarpaulin XML parsing

    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_badge_url_generation() {
        let url = generate_badge_url(85.0);
        assert!(url.contains("brightgreen"));
        assert!(url.contains("85"));
    }

    #[test]
    fn test_ci_coverage_check() {
        let integration = CICoverageIntegration::new(80.0);
        let result = integration.check_coverage();
        assert!(result.report_markdown.contains("Coverage Report"));
    }
}
