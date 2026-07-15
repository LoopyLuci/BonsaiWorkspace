//! Coverage CLI - records a couple of sample crate results and prints the CI check

use coverage::{CICoverageIntegration, CoverageData};

fn main() {
    let integration = CICoverageIntegration::new(80.0);

    integration.record_coverage(vec![
        CoverageData {
            crate_name: "example-a".to_string(),
            lines_covered: 92,
            lines_total: 100,
            branch_coverage: 88.0,
            coverage_percent: 92.0,
            files: vec![],
        },
        CoverageData {
            crate_name: "example-b".to_string(),
            lines_covered: 55,
            lines_total: 100,
            branch_coverage: 40.0,
            coverage_percent: 55.0,
            files: vec![],
        },
    ]);

    let result = integration.check_coverage();
    println!("passed: {}", result.passed);
    println!("overall coverage: {:.2}%", result.coverage_percent);
    println!("{}", result.report_markdown);
}
