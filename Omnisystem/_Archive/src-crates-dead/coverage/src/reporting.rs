use crate::collector::CoverageResult;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Coverage report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub timestamp: DateTime<Utc>,
    pub overall_coverage_percent: f64,
    pub total_crates: usize,
    pub crates_above_target: usize,
    pub crates_below_target: usize,
    pub target_coverage: f64,
    pub crate_reports: Vec<CrateReport>,
    pub recommendations: Vec<String>,
}

/// Per-crate report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateReport {
    pub crate_name: String,
    pub coverage_percent: f64,
    pub lines_covered: usize,
    pub lines_total: usize,
    pub target_met: bool,
    pub change_percent: Option<f64>,
}

/// Generates coverage reports
pub struct CoverageReporter {
    target_coverage: f64,
}

impl CoverageReporter {
    pub fn new(target_coverage: f64) -> Self {
        Self { target_coverage }
    }

    /// Generate report from results
    pub fn generate(
        &self,
        results: &[CoverageResult],
        previous_results: Option<&[CoverageResult]>,
    ) -> CoverageReport {
        if results.is_empty() {
            return self.empty_report();
        }

        let total_covered: usize = results.iter().map(|r| r.lines_covered).sum();
        let total_lines: usize = results.iter().map(|r| r.lines_total).sum();

        let overall = if total_lines == 0 {
            100.0
        } else {
            (total_covered as f64 / total_lines as f64) * 100.0
        };

        let mut crate_reports = Vec::new();
        let mut above_target = 0;
        let mut below_target = 0;

        for result in results {
            let target_met = result.coverage_percent >= self.target_coverage;
            if target_met {
                above_target += 1;
            } else {
                below_target += 1;
            }

            let change = self.calculate_change(result, previous_results);

            crate_reports.push(CrateReport {
                crate_name: result.crate_name.clone(),
                coverage_percent: result.coverage_percent,
                lines_covered: result.lines_covered,
                lines_total: result.lines_total,
                target_met,
                change_percent: change,
            });
        }

        // Sort by coverage (lowest first)
        crate_reports.sort_by(|a, b| {
            a.coverage_percent
                .partial_cmp(&b.coverage_percent)
                .unwrap()
        });

        let recommendations = self.generate_recommendations(&crate_reports, overall);

        CoverageReport {
            timestamp: Utc::now(),
            overall_coverage_percent: overall,
            total_crates: results.len(),
            crates_above_target: above_target,
            crates_below_target: below_target,
            target_coverage: self.target_coverage,
            crate_reports,
            recommendations,
        }
    }

    /// Export report as markdown
    pub fn export_markdown(&self, report: &CoverageReport) -> String {
        let mut md = String::new();

        md.push_str("# Coverage Report\n\n");
        md.push_str(&format!(
            "**Generated:** {}\n\n",
            report.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        // Summary
        md.push_str("## Summary\n\n");
        md.push_str(&format!(
            "| Metric | Value |\n|--------|-------|\n"
        ));
        md.push_str(&format!(
            "| Overall Coverage | {:.2}% |\n",
            report.overall_coverage_percent
        ));
        md.push_str(&format!(
            "| Target Coverage | {:.2}% |\n",
            report.target_coverage
        ));
        md.push_str(&format!(
            "| Crates Above Target | {} |\n",
            report.crates_above_target
        ));
        md.push_str(&format!(
            "| Crates Below Target | {} |\n\n",
            report.crates_below_target
        ));

        // Crate details
        md.push_str("## Per-Crate Coverage\n\n");
        md.push_str(
            "| Crate | Coverage | Lines | Target | Change |\n|-------|----------|-------|--------|--------|\n",
        );

        for crate_report in &report.crate_reports {
            let status = if crate_report.target_met { "✓" } else { "✗" };
            let change_str = if let Some(change) = crate_report.change_percent {
                format!("{:+.2}%", change)
            } else {
                "—".to_string()
            };

            md.push_str(&format!(
                "| {} {} | {:.2}% | {}/{} | {:.2}% | {} |\n",
                status,
                crate_report.crate_name,
                crate_report.coverage_percent,
                crate_report.lines_covered,
                crate_report.lines_total,
                self.target_coverage,
                change_str
            ));
        }

        // Recommendations
        if !report.recommendations.is_empty() {
            md.push_str("\n## Recommendations\n\n");
            for (i, rec) in report.recommendations.iter().enumerate() {
                md.push_str(&format!("{}. {}\n", i + 1, rec));
            }
        }

        md
    }

    /// Export report as JSON
    pub fn export_json(&self, report: &CoverageReport) -> Result<String, String> {
        serde_json::to_string_pretty(report)
            .map_err(|e| format!("JSON export failed: {}", e))
    }

    fn empty_report(&self) -> CoverageReport {
        CoverageReport {
            timestamp: Utc::now(),
            overall_coverage_percent: 0.0,
            total_crates: 0,
            crates_above_target: 0,
            crates_below_target: 0,
            target_coverage: self.target_coverage,
            crate_reports: Vec::new(),
            recommendations: vec!["No coverage data available".to_string()],
        }
    }

    fn calculate_change(
        &self,
        result: &CoverageResult,
        previous_results: Option<&[CoverageResult]>,
    ) -> Option<f64> {
        if let Some(previous) = previous_results {
            previous
                .iter()
                .find(|p| p.crate_name == result.crate_name)
                .map(|p| result.coverage_percent - p.coverage_percent)
        } else {
            None
        }
    }

    fn generate_recommendations(&self, crate_reports: &[CrateReport], overall: f64) -> Vec<String> {
        let mut recs = Vec::new();

        let below_target: Vec<_> = crate_reports
            .iter()
            .filter(|c| !c.target_met)
            .collect();

        if !below_target.is_empty() {
            recs.push(format!(
                "Focus on {}: {} crate(s) below {}% target",
                below_target[0].crate_name,
                below_target.len(),
                self.target_coverage as i32
            ));
        }

        if overall < (self.target_coverage - 5.0) {
            recs.push("Overall coverage is significantly below target. Prioritize test additions.".to_string());
        }

        if below_target.len() > crate_reports.len() / 2 {
            recs.push("More than 50% of crates are below target. Consider team-wide testing initiative.".to_string());
        }

        recs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_export() {
        let reporter = CoverageReporter::new(80.0);
        let report = reporter.generate(&[], None);
        let md = reporter.export_markdown(&report);
        assert!(md.contains("Coverage Report"));
    }

    #[test]
    fn test_json_export() {
        let reporter = CoverageReporter::new(80.0);
        let report = reporter.generate(&[], None);
        let json = reporter.export_json(&report);
        assert!(json.is_ok());
    }
}
