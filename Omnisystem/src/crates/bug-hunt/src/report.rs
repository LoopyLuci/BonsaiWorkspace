/// Report generation in multiple formats: JSON, SARIF, HTML, Markdown.

use anyhow::Result;
use serde_json::json;
use std::fs;
use std::path::Path;

use crate::finding::{Finding, ScanReport, Severity};

/// Report generator supporting multiple output formats.
pub struct ReportGenerator;

impl ReportGenerator {
    /// Generate a JSON report.
    pub fn to_json(report: &ScanReport) -> Result<String> {
        let json = serde_json::to_string_pretty(&report)?;
        Ok(json)
    }

    /// Generate a SARIF (Static Analysis Results Format) report.
    pub fn to_sarif(report: &ScanReport) -> Result<String> {
        let mut results = Vec::new();

        for issue in &report.issues {
            let result = json!({
                "ruleId": issue.rule_id,
                "level": issue.severity.as_str(),
                "message": {
                    "text": &issue.message,
                    "markdown": format!("**{}**: {}\n\nSuggestion: {}",
                        issue.rule_id,
                        issue.message,
                        issue.suggestion.as_deref().unwrap_or("N/A")
                    )
                },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": {
                            "uri": issue.file_path.to_string_lossy().to_string()
                        },
                        "region": {
                            "startLine": issue.line_start,
                            "endLine": issue.line_end,
                            "startColumn": issue.column_start.unwrap_or(0),
                            "endColumn": issue.column_end.unwrap_or(0)
                        }
                    }
                }],
                "taxa": [
                    {
                        "id": "potential-issue",
                        "index": 0
                    }
                ]
            });
            results.push(result);
        }

        let sarif = json!({
            "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
            "version": "2.1.0",
            "runs": [{
                "tool": {
                    "driver": {
                        "name": "Bonsai Bug Hunt",
                        "version": "0.1.0",
                        "informationUri": "https://github.com/bonsai/bonsai-ecosystem"
                    }
                },
                "results": results
            }]
        });

        Ok(serde_json::to_string_pretty(&sarif)?)
    }

    /// Generate a Markdown report.
    pub fn to_markdown(report: &ScanReport) -> Result<String> {
        let mut md = String::new();

        md.push_str(&format!("# Bug Hunt Report\n\n"));
        md.push_str(&format!("**Repository:** {}\n", report.summary.repository));
        md.push_str(&format!("**Scan ID:** {}\n", report.summary.scan_id));
        md.push_str(&format!("**Timestamp:** {}\n\n", report.summary.timestamp));

        md.push_str("## Summary\n\n");
        md.push_str(&format!("- **Files Scanned:** {}\n", report.summary.files_scanned));
        md.push_str(&format!("- **Total Issues:** {}\n", report.summary.issues_found));
        md.push_str(&format!("  - 🔴 Critical: {}\n", report.summary.critical));
        md.push_str(&format!("  - 🟠 High: {}\n", report.summary.high));
        md.push_str(&format!("  - 🟡 Medium: {}\n", report.summary.medium));
        md.push_str(&format!("  - 🔵 Low: {}\n", report.summary.low));
        md.push_str(&format!("  - ⚪ Info: {}\n\n", report.summary.info));

        if report.issues.is_empty() {
            md.push_str("✅ No issues found!\n");
            return Ok(md);
        }

        md.push_str("## Issues\n\n");

        for issue in &report.issues {
            let severity_emoji = match issue.severity {
                Severity::Critical => "🔴",
                Severity::High => "🟠",
                Severity::Medium => "🟡",
                Severity::Low => "🔵",
                Severity::Info => "⚪",
            };

            md.push_str(&format!(
                "### {} {} - {}\n\n",
                severity_emoji, issue.severity.as_str(), issue.rule_id
            ));
            md.push_str(&format!("**File:** `{}`\n", issue.file_path.display()));
            md.push_str(&format!("**Line:** {}-{}\n", issue.line_start, issue.line_end));
            md.push_str(&format!("**Analyzer:** {}\n", issue.analyzer));
            md.push_str(&format!("**Confidence:** {:.0}%\n\n", issue.confidence * 100.0));

            md.push_str(&format!("**Message:**\n```\n{}\n```\n\n", issue.message));

            if let Some(suggestion) = &issue.suggestion {
                md.push_str(&format!("**Suggestion:**\n{}\n\n", suggestion));
            }

            if let Some(diff) = &issue.suggested_diff {
                md.push_str("**Suggested Fix:**\n```diff\n");
                md.push_str(diff);
                md.push_str("\n```\n\n");
            }

            if !issue.tags.is_empty() {
                md.push_str(&format!("**Tags:** {}\n\n", issue.tags.join(", ")));
            }
        }

        Ok(md)
    }

    /// Generate an HTML report.
    pub fn to_html(report: &ScanReport) -> Result<String> {
        let mut html = String::from(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Bug Hunt Report</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }
        .container { max-width: 1200px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }
        h1 { color: #333; border-bottom: 3px solid #007bff; padding-bottom: 10px; }
        h2 { color: #555; margin-top: 30px; }
        .summary { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 15px; margin: 20px 0; }
        .summary-box { padding: 15px; border-radius: 6px; text-align: center; }
        .critical { background: #ffe0e0; color: #c00; font-weight: bold; }
        .high { background: #fff3e0; color: #ff6f00; font-weight: bold; }
        .medium { background: #fffde7; color: #f57f17; font-weight: bold; }
        .low { background: #e3f2fd; color: #1976d2; }
        .info { background: #f3e5f5; color: #6a1b9a; }
        .issue { border-left: 4px solid #ddd; padding: 15px; margin: 15px 0; background: #fafafa; border-radius: 4px; }
        .issue.critical { border-left-color: #c00; background: #fff5f5; }
        .issue.high { border-left-color: #ff6f00; background: #fff8f3; }
        .issue.medium { border-left-color: #f57f17; background: #fffbf0; }
        .issue.low { border-left-color: #1976d2; background: #f3f8ff; }
        .issue-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; }
        .rule-id { font-weight: bold; color: #333; }
        .severity { padding: 4px 8px; border-radius: 4px; font-size: 0.85em; }
        .file-path { color: #666; font-family: monospace; font-size: 0.9em; }
        .message { background: white; padding: 10px; border-radius: 4px; border-left: 3px solid #ddd; font-family: monospace; }
        .suggestion { background: #f0f8ff; padding: 10px; border-radius: 4px; margin: 10px 0; }
        .confidence { color: #999; font-size: 0.85em; }
    </style>
</head>
<body>
    <div class="container">
"#);

        html.push_str(&format!("<h1>Bug Hunt Report</h1>\n"));
        html.push_str(&format!(
            "<p><strong>Repository:</strong> {}<br>\n",
            report.summary.repository
        ));
        html.push_str(&format!(
            "<strong>Scan ID:</strong> {}<br>\n",
            report.summary.scan_id
        ));
        html.push_str(&format!(
            "<strong>Timestamp:</strong> {}</p>\n",
            report.summary.timestamp
        ));

        html.push_str("<h2>Summary</h2>\n<div class=\"summary\">\n");
        html.push_str(&format!(
            "<div class=\"summary-box\">Files Scanned<br>{}</div>\n",
            report.summary.files_scanned
        ));
        html.push_str(&format!(
            "<div class=\"summary-box critical\">Critical<br>{}</div>\n",
            report.summary.critical
        ));
        html.push_str(&format!(
            "<div class=\"summary-box high\">High<br>{}</div>\n",
            report.summary.high
        ));
        html.push_str(&format!(
            "<div class=\"summary-box medium\">Medium<br>{}</div>\n",
            report.summary.medium
        ));
        html.push_str(&format!(
            "<div class=\"summary-box low\">Low<br>{}</div>\n",
            report.summary.low
        ));
        html.push_str("</div>\n");

        if !report.issues.is_empty() {
            html.push_str("<h2>Issues</h2>\n");

            for issue in &report.issues {
                let severity_str = issue.severity.as_str();
                html.push_str(&format!(
                    "<div class=\"issue {}\"><div class=\"issue-header\">\n",
                    severity_str
                ));
                html.push_str(&format!("<div><span class=\"rule-id\">{}</span></div>\n", issue.rule_id));
                html.push_str(&format!(
                    "<span class=\"severity {}\">{}</span>\n",
                    severity_str, severity_str
                ));
                html.push_str("</div>\n");

                html.push_str(&format!(
                    "<p class=\"file-path\">{}:{}-{}</p>\n",
                    issue.file_path.display(),
                    issue.line_start,
                    issue.line_end
                ));

                html.push_str(&format!("<div class=\"message\">{}</div>\n", issue.message));

                if let Some(suggestion) = &issue.suggestion {
                    html.push_str(&format!("<div class=\"suggestion\"><strong>Suggestion:</strong> {}</div>\n", suggestion));
                }

                html.push_str(&format!(
                    "<p class=\"confidence\">Analyzer: {} | Confidence: {:.0}%</p>\n",
                    issue.analyzer,
                    issue.confidence * 100.0
                ));

                html.push_str("</div>\n");
            }
        } else {
            html.push_str("<p>✅ No issues found!</p>\n");
        }

        html.push_str("    </div>\n</body>\n</html>\n");

        Ok(html)
    }

    /// Write a report to a file.
    pub fn write_to_file(report: &ScanReport, path: &Path, format: &str) -> Result<()> {
        let content = match format {
            "json" => Self::to_json(report)?,
            "sarif" => Self::to_sarif(report)?,
            "html" => Self::to_html(report)?,
            "markdown" => Self::to_markdown(report)?,
            "md" => Self::to_markdown(report)?,
            _ => return Err(anyhow::anyhow!("Unsupported format: {}", format)),
        };

        fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::ScanSummary;

    #[test]
    fn test_report_generation() -> Result<()> {
        let summary = ScanSummary {
            scan_id: "test-scan".to_string(),
            repository: "test-repo".to_string(),
            timestamp: chrono::Utc::now(),
            files_scanned: 10,
            issues_found: 0,
            critical: 0,
            high: 0,
            medium: 0,
            low: 0,
            info: 0,
        };

        let report = ScanReport {
            summary,
            issues: Vec::new(),
        };

        let json = ReportGenerator::to_json(&report)?;
        assert!(json.contains("test-repo"));

        let md = ReportGenerator::to_markdown(&report)?;
        assert!(md.contains("No issues found"));

        Ok(())
    }
}
