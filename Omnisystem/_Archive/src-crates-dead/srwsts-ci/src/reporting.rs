//! Report generation with side-by-side metrics comparison and trend analysis

use crate::baseline::Baseline;
use crate::detection::RegressionFinding;
use crate::errors::CIResult;
use crate::metrics::PerformanceMetrics;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;

/// Single metric comparison entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricComparison {
    pub metric_name: String,
    pub baseline_value: f64,
    pub current_value: f64,
    pub difference: f64,
    pub difference_percent: f64,
    pub trend: Trend,
}

/// Trend direction
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum Trend {
    Improved,
    Degraded,
    Stable,
}

impl MetricComparison {
    pub fn new(
        metric_name: &str,
        baseline_value: f64,
        current_value: f64,
    ) -> Self {
        let difference = current_value - baseline_value;
        let difference_percent = if baseline_value == 0.0 {
            0.0
        } else {
            (difference / baseline_value) * 100.0
        };

        let trend = if difference.abs() < 0.1 {
            Trend::Stable
        } else if difference < 0.0 {
            if metric_name.contains("latency") || metric_name.contains("memory") {
                Trend::Improved
            } else {
                Trend::Degraded
            }
        } else if metric_name.contains("latency") || metric_name.contains("memory") {
            Trend::Degraded
        } else {
            Trend::Improved
        };

        Self {
            metric_name: metric_name.to_string(),
            baseline_value,
            current_value,
            difference,
            difference_percent,
            trend,
        }
    }
}

/// Complete regression report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionReport {
    pub generated_at: DateTime<Utc>,
    pub baseline_version: String,
    pub baseline_commit: String,
    pub current_commit: String,
    pub metrics_comparisons: Vec<MetricComparison>,
    pub regression_findings: Vec<RegressionFinding>,
    pub summary: ReportSummary,
    pub recommendations: Vec<String>,
    pub html_report: Option<String>,
}

/// Report summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_metrics: usize,
    pub improved_metrics: usize,
    pub degraded_metrics: usize,
    pub stable_metrics: usize,
    pub total_regressions: usize,
    pub critical_regressions: usize,
    pub warning_regressions: usize,
    pub overall_status: String,
}

/// Report generator
pub struct ReportGenerator;

impl ReportGenerator {
    /// Generate comprehensive regression report
    pub fn generate(
        baseline: &Baseline,
        current_metrics: &PerformanceMetrics,
        regression_findings: &[RegressionFinding],
        current_commit: &str,
    ) -> CIResult<RegressionReport> {
        let baseline_metrics = PerformanceMetrics::from_snapshot(&baseline.metrics)?;

        // Build metric comparisons
        let mut comparisons = Vec::new();
        let all_metrics = baseline_metrics.as_flat_map();

        for (metric_name, baseline_value) in all_metrics {
            let current_value = current_metrics.get_metric(&metric_name).unwrap_or(0.0);
            comparisons.push(MetricComparison::new(&metric_name, baseline_value, current_value));
        }

        // Calculate summary
        let improved = comparisons.iter().filter(|c| c.trend == Trend::Improved).count();
        let degraded = comparisons.iter().filter(|c| c.trend == Trend::Degraded).count();
        let stable = comparisons.iter().filter(|c| c.trend == Trend::Stable).count();

        let critical = regression_findings
            .iter()
            .filter(|f| matches!(f.severity, crate::detection::RegressionSeverity::Critical))
            .count();
        let warnings = regression_findings
            .iter()
            .filter(|f| matches!(f.severity, crate::detection::RegressionSeverity::Warning))
            .count();

        let overall_status = if critical > 0 {
            "FAILED".to_string()
        } else if !regression_findings.is_empty() {
            "WARNING".to_string()
        } else {
            "PASSED".to_string()
        };

        let summary = ReportSummary {
            total_metrics: comparisons.len(),
            improved_metrics: improved,
            degraded_metrics: degraded,
            stable_metrics: stable,
            total_regressions: regression_findings.len(),
            critical_regressions: critical,
            warning_regressions: warnings,
            overall_status,
        };

        // Generate recommendations
        let recommendations = Self::generate_recommendations(regression_findings);

        let report = RegressionReport {
            generated_at: Utc::now(),
            baseline_version: baseline.version.version.clone(),
            baseline_commit: baseline.version.commit_hash.clone(),
            current_commit: current_commit.to_string(),
            metrics_comparisons: comparisons,
            regression_findings: regression_findings.to_vec(),
            summary,
            recommendations,
            html_report: None,
        };

        info!("Report generated: {} regressions found", regression_findings.len());
        Ok(report)
    }

    /// Generate recommendations based on findings
    fn generate_recommendations(findings: &[RegressionFinding]) -> Vec<String> {
        let mut recommendations = Vec::new();

        let critical_count = findings
            .iter()
            .filter(|f| matches!(f.severity, crate::detection::RegressionSeverity::Critical))
            .count();
        let perf_count = findings
            .iter()
            .filter(|f| f.finding_type == crate::detection::RegressionType::Performance)
            .count();
        let correctness_count = findings
            .iter()
            .filter(|f| f.finding_type == crate::detection::RegressionType::Correctness)
            .count();

        if critical_count > 0 {
            recommendations.push("BLOCK MERGE: Critical regression detected. Investigate and fix before merging.".to_string());
        }

        if correctness_count > 0 {
            recommendations.push(
                "URGENT: Correctness regressions detected. Tests that previously passed are now failing."
                    .to_string(),
            );
            recommendations.push(
                "Action: Run local tests, check recent code changes, revert if necessary.".to_string(),
            );
        }

        if perf_count > 0 {
            recommendations.push(
                "Performance regression detected. Evaluate if this is acceptable or requires optimization."
                    .to_string(),
            );
            if perf_count > 3 {
                recommendations.push(
                    "Action: Profile hotspots, consider algorithmic improvements or cache optimization."
                        .to_string(),
                );
            }
        }

        if findings.iter().all(|f| matches!(f.severity, crate::detection::RegressionSeverity::Warning)) {
            recommendations.push("Minor regressions detected but within acceptable bounds.".to_string());
            recommendations.push(
                "Action: Monitor in nightly tests, consider optimization in next sprint.".to_string(),
            );
        }

        if findings.is_empty() {
            recommendations.push("All metrics stable. No regressions detected.".to_string());
            recommendations.push("Status: READY TO MERGE".to_string());
        }

        recommendations
    }

    /// Generate HTML report for web viewing
    pub fn generate_html(report: &RegressionReport) -> String {
        let mut html = String::new();

        html.push_str("<!DOCTYPE html>\n");
        html.push_str("<html>\n<head>\n");
        html.push_str("<meta charset=\"UTF-8\">\n");
        html.push_str("<title>Regression Report</title>\n");
        html.push_str("<style>\n");
        html.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
        html.push_str("table { border-collapse: collapse; width: 100%; margin: 20px 0; }\n");
        html.push_str("th, td { border: 1px solid #ddd; padding: 12px; text-align: left; }\n");
        html.push_str("th { background-color: #f2f2f2; }\n");
        html.push_str(".status-pass { color: green; font-weight: bold; }\n");
        html.push_str(".status-warn { color: orange; font-weight: bold; }\n");
        html.push_str(".status-fail { color: red; font-weight: bold; }\n");
        html.push_str(".improved { color: green; }\n");
        html.push_str(".degraded { color: red; }\n");
        html.push_str(".stable { color: gray; }\n");
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");

        // Title and summary
        let status_class = match report.summary.overall_status.as_str() {
            "PASSED" => "status-pass",
            "WARNING" => "status-warn",
            "FAILED" => "status-fail",
            _ => "status-warn",
        };

        html.push_str(&format!(
            "<h1>Regression Report <span class=\"{}\">{}</span></h1>\n",
            status_class, report.summary.overall_status
        ));
        html.push_str(&format!(
            "<p>Generated: {}</p>\n",
            report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        html.push_str(&format!(
            "<p>Baseline: {} ({})</p>\n",
            report.baseline_version, report.baseline_commit
        ));
        html.push_str(&format!("<p>Current: {}</p>\n", report.current_commit));

        // Summary table
        html.push_str("<h2>Summary</h2>\n");
        html.push_str("<table>\n");
        html.push_str("<tr><th>Metric</th><th>Count</th></tr>\n");
        html.push_str(&format!(
            "<tr><td>Total Metrics</td><td>{}</td></tr>\n",
            report.summary.total_metrics
        ));
        html.push_str(&format!(
            "<tr><td>Improved</td><td class=\"improved\">{}</td></tr>\n",
            report.summary.improved_metrics
        ));
        html.push_str(&format!(
            "<tr><td>Degraded</td><td class=\"degraded\">{}</td></tr>\n",
            report.summary.degraded_metrics
        ));
        html.push_str(&format!(
            "<tr><td>Stable</td><td class=\"stable\">{}</td></tr>\n",
            report.summary.stable_metrics
        ));
        html.push_str(&format!(
            "<tr><td>Total Regressions</td><td class=\"degraded\">{}</td></tr>\n",
            report.summary.total_regressions
        ));
        html.push_str("</table>\n");

        // Metrics comparison table
        html.push_str("<h2>Metrics Comparison</h2>\n");
        html.push_str("<table>\n");
        html.push_str(
            "<tr><th>Metric</th><th>Baseline</th><th>Current</th><th>Difference</th><th>Trend</th></tr>\n",
        );
        for comp in &report.metrics_comparisons {
            let trend_class = match comp.trend {
                Trend::Improved => "improved",
                Trend::Degraded => "degraded",
                Trend::Stable => "stable",
            };
            html.push_str(&format!(
                "<tr><td>{}</td><td>{:.2}</td><td>{:.2}</td><td>{:.2}%</td><td class=\"{}\">{:?}</td></tr>\n",
                comp.metric_name, comp.baseline_value, comp.current_value, comp.difference_percent, trend_class, comp.trend
            ));
        }
        html.push_str("</table>\n");

        // Regressions table
        if !report.regression_findings.is_empty() {
            html.push_str("<h2>Regression Findings</h2>\n");
            html.push_str("<table>\n");
            html.push_str(
                "<tr><th>Metric</th><th>Type</th><th>Severity</th><th>Difference</th><th>Message</th></tr>\n",
            );
            for finding in &report.regression_findings {
                let severity_class = match finding.severity {
                    crate::detection::RegressionSeverity::Warning => "status-warn",
                    crate::detection::RegressionSeverity::Failure => "status-fail",
                    crate::detection::RegressionSeverity::Critical => "status-fail",
                };
                html.push_str(&format!(
                    "<tr><td>{}</td><td>{:?}</td><td class=\"{}\">{:?}</td><td>{:.2}%</td><td>{}</td></tr>\n",
                    finding.metric, finding.finding_type, severity_class, finding.severity, finding.difference_percent, finding.message
                ));
            }
            html.push_str("</table>\n");
        }

        // Recommendations
        if !report.recommendations.is_empty() {
            html.push_str("<h2>Recommendations</h2>\n");
            html.push_str("<ul>\n");
            for rec in &report.recommendations {
                html.push_str(&format!("<li>{}</li>\n", rec));
            }
            html.push_str("</ul>\n");
        }

        html.push_str("</body>\n</html>\n");
        html
    }

    /// Generate markdown report
    pub fn generate_markdown(report: &RegressionReport) -> String {
        let mut md = String::new();

        md.push_str("# Regression Report\n\n");
        md.push_str(&format!("**Status**: {}\n\n", report.summary.overall_status));
        md.push_str(&format!(
            "Generated: {}\n\n",
            report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        md.push_str(&format!(
            "**Baseline**: {} ({})\n\n",
            report.baseline_version, report.baseline_commit
        ));
        md.push_str(&format!("**Current**: {}\n\n", report.current_commit));

        md.push_str("## Summary\n\n");
        md.push_str(&format!(
            "| Metric | Count |\n| --- | --- |\n"
        ));
        md.push_str(&format!("| Total Metrics | {} |\n", report.summary.total_metrics));
        md.push_str(&format!(
            "| Improved | {} |\n",
            report.summary.improved_metrics
        ));
        md.push_str(&format!(
            "| Degraded | {} |\n",
            report.summary.degraded_metrics
        ));
        md.push_str(&format!("| Stable | {} |\n", report.summary.stable_metrics));
        md.push_str(&format!(
            "| Regressions | {} |\n\n",
            report.summary.total_regressions
        ));

        if !report.regression_findings.is_empty() {
            md.push_str("## Regression Findings\n\n");
            for finding in &report.regression_findings {
                md.push_str(&format!(
                    "- **{}**: {} ({:?})\n",
                    finding.metric, finding.message, finding.severity
                ));
            }
            md.push_str("\n");
        }

        if !report.recommendations.is_empty() {
            md.push_str("## Recommendations\n\n");
            for rec in &report.recommendations {
                md.push_str(&format!("- {}\n", rec));
            }
            md.push_str("\n");
        }

        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::{Baseline, BaselineIntegrity, BaselineVersion, TestResult};
    use crate::detection::RegressionFinding;
    use crate::metrics::{MetricsSnapshot, PerformanceMetrics};

    fn create_baseline() -> Baseline {
        let mut snapshot = MetricsSnapshot::new();
        for i in 0..10 {
            snapshot.add_latency(100.0 + i as f64);
            snapshot.add_throughput(1000.0 + i as f64 * 10.0);
        }
        snapshot.set_memory_peak(512.0);
        snapshot.set_cpu_average(45.0);
        snapshot.set_io_operations(42);

        Baseline {
            version: BaselineVersion {
                version: "1.0.0".to_string(),
                commit_hash: "abc123".to_string(),
                timestamp: Utc::now(),
                approved_by: None,
            },
            metrics: snapshot,
            test_results: {
                let mut map = HashMap::new();
                map.insert(
                    "test_basic".to_string(),
                    TestResult {
                        test_name: "test_basic".to_string(),
                        passed: true,
                        duration_ms: 100,
                        error_message: None,
                        determinism_runs: vec![100, 101, 100],
                    },
                );
                map
            },
            integrity: BaselineIntegrity {
                content_hash: "def".to_string(),
                metadata_hash: "ghi".to_string(),
                computed_at: Utc::now(),
                verified: true,
            },
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_metric_comparison_creation() {
        let comp = MetricComparison::new("latency_p99", 100.0, 110.0);
        assert_eq!(comp.metric_name, "latency_p99");
        assert_eq!(comp.difference_percent, 10.0);
        assert_eq!(comp.trend, Trend::Degraded);
    }

    #[test]
    fn test_metric_comparison_improvement() {
        let comp = MetricComparison::new("latency_p99", 100.0, 90.0);
        assert_eq!(comp.trend, Trend::Improved);
    }

    #[test]
    fn test_report_generation() {
        let baseline = create_baseline();
        let mut current_snapshot = MetricsSnapshot::new();
        for i in 0..10 {
            current_snapshot.add_latency(100.0 + i as f64);
            current_snapshot.add_throughput(1000.0 + i as f64 * 10.0);
        }
        current_snapshot.set_memory_peak(512.0);
        current_snapshot.set_cpu_average(45.0);
        current_snapshot.set_io_operations(42);

        let current_metrics = PerformanceMetrics::from_snapshot(&current_snapshot).unwrap();

        let report = ReportGenerator::generate(&baseline, &current_metrics, &[], "def456").unwrap();

        assert_eq!(report.baseline_version, "1.0.0");
        assert_eq!(report.current_commit, "def456");
        assert_eq!(report.summary.total_regressions, 0);
        assert_eq!(report.summary.overall_status, "PASSED");
    }

    #[test]
    fn test_report_with_regressions() {
        let baseline = create_baseline();
        let mut current_snapshot = MetricsSnapshot::new();
        for i in 0..10 {
            current_snapshot.add_latency(120.0 + i as f64);
            current_snapshot.add_throughput(900.0 + i as f64 * 10.0);
        }
        current_snapshot.set_memory_peak(512.0);
        current_snapshot.set_cpu_average(45.0);
        current_snapshot.set_io_operations(42);

        let current_metrics = PerformanceMetrics::from_snapshot(&current_snapshot).unwrap();

        let findings = vec![RegressionFinding {
            finding_type: crate::detection::RegressionType::Performance,
            severity: crate::detection::RegressionSeverity::Failure,
            metric: "latency_p99".to_string(),
            baseline_value: 100.0,
            current_value: 120.0,
            difference_percent: 20.0,
            threshold_percent: 5.0,
            message: "Latency regressed".to_string(),
        }];

        let report = ReportGenerator::generate(&baseline, &current_metrics, &findings, "def456").unwrap();

        assert_eq!(report.summary.total_regressions, 1);
        assert_eq!(report.summary.overall_status, "FAILED");
        assert!(!report.recommendations.is_empty());
    }

    #[test]
    fn test_generate_html_report() {
        let baseline = create_baseline();
        let mut snapshot = MetricsSnapshot::new();
        for i in 0..5 {
            snapshot.add_latency(100.0 + i as f64);
            snapshot.add_throughput(1000.0 + i as f64 * 10.0);
        }
        snapshot.set_memory_peak(512.0);
        snapshot.set_cpu_average(45.0);
        snapshot.set_io_operations(42);

        let metrics = PerformanceMetrics::from_snapshot(&snapshot).unwrap();
        let report = ReportGenerator::generate(&baseline, &metrics, &[], "def456").unwrap();

        let html = ReportGenerator::generate_html(&report);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Regression Report"));
        assert!(html.contains("PASSED"));
    }

    #[test]
    fn test_generate_markdown_report() {
        let baseline = create_baseline();
        let mut snapshot = MetricsSnapshot::new();
        for i in 0..5 {
            snapshot.add_latency(100.0 + i as f64);
            snapshot.add_throughput(1000.0 + i as f64 * 10.0);
        }
        snapshot.set_memory_peak(512.0);
        snapshot.set_cpu_average(45.0);
        snapshot.set_io_operations(42);

        let metrics = PerformanceMetrics::from_snapshot(&snapshot).unwrap();
        let report = ReportGenerator::generate(&baseline, &metrics, &[], "def456").unwrap();

        let markdown = ReportGenerator::generate_markdown(&report);
        assert!(markdown.contains("# Regression Report"));
        assert!(markdown.contains("PASSED"));
    }

    #[test]
    fn test_recommendations_generation() {
        let findings = vec![
            RegressionFinding {
                finding_type: crate::detection::RegressionType::Correctness,
                severity: crate::detection::RegressionSeverity::Failure,
                metric: "test_basic".to_string(),
                baseline_value: 1.0,
                current_value: 0.0,
                difference_percent: -100.0,
                threshold_percent: 0.0,
                message: "Test failed".to_string(),
            },
        ];

        let recs = ReportGenerator::generate_recommendations(&findings);
        assert!(!recs.is_empty());
        assert!(recs.iter().any(|r| r.contains("BLOCK")));
    }
}
