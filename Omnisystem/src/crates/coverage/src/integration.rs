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

/// Parse tarpaulin XML (Cobertura format) output.
///
/// Extracts per-class `line-rate` attributes and aggregates them into
/// per-crate coverage. This is intentionally a light-weight string scan
/// rather than a full XML parser (no third-party XML dependency in this
/// crate); it is tolerant of the exact Cobertura schema tarpaulin emits
/// but not a general-purpose XML parser.
pub fn parse_tarpaulin_output(xml_content: &str) -> Result<Vec<CoverageData>, String> {
    let mut results = Vec::new();

    for line in xml_content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("<class ") {
            continue;
        }

        let name = extract_xml_attr(trimmed, "name").unwrap_or_else(|| "unknown".to_string());
        let line_rate: f64 = extract_xml_attr(trimmed, "line-rate")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        let coverage_percent = line_rate * 100.0;
        results.push(CoverageData {
            crate_name: name,
            lines_covered: 0,
            lines_total: 0,
            branch_coverage: 0.0,
            coverage_percent,
            files: vec![],
        });
    }

    Ok(results)
}

fn extract_xml_attr(tag: &str, attr: &str) -> Option<String> {
    let needle = format!("{attr}=\"");
    let start = tag.find(&needle)? + needle.len();
    let rest = &tag[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
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

    #[test]
    fn test_parse_tarpaulin_output() {
        let xml = r#"<?xml version="1.0" ?>
<coverage>
  <packages>
    <package name="root">
      <classes>
        <class name="my-crate" filename="src/lib.rs" line-rate="0.85" branch-rate="0.7">
        </class>
        <class name="other-crate" filename="src/main.rs" line-rate="0.42" branch-rate="0.3">
        </class>
      </classes>
    </package>
  </packages>
</coverage>"#;

        let results = parse_tarpaulin_output(xml).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].crate_name, "my-crate");
        assert!((results[0].coverage_percent - 85.0).abs() < 0.01);
        assert_eq!(results[1].crate_name, "other-crate");
        assert!((results[1].coverage_percent - 42.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_tarpaulin_output_empty() {
        let results = parse_tarpaulin_output("<coverage></coverage>").unwrap();
        assert!(results.is_empty());
    }
}
