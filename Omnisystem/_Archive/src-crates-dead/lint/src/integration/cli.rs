/// CLI interface for the linter: `bonsai lint`

use crate::engine::LintConfig;
use crate::diagnostics::Diagnostic;
use anyhow::Result;
use serde_json::json;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CliArgs {
    pub command: LintCommand,
}

#[derive(Debug, Clone)]
pub enum LintCommand {
    Lint {
        path: Option<PathBuf>,
        exclude: Vec<String>,
        output_format: OutputFormat,
        confidence_threshold: f32,
    },
    Check {
        path: PathBuf,
    },
    GenerateRule {
        description: String,
        language: String,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Json,
    Sarif,
    Html,
    Terminal,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Self {
        match s {
            "json" => Self::Json,
            "sarif" => Self::Sarif,
            "html" => Self::Html,
            _ => Self::Terminal,
        }
    }
}

pub fn format_diagnostics(diagnostics: &[Diagnostic], format: OutputFormat) -> Result<String> {
    match format {
        OutputFormat::Json => {
            let json = json!({
                "version": "1.0.0",
                "runs": [{
                    "results": diagnostics.iter().map(|d| json!({
                        "ruleId": d.rule_id,
                        "message": { "text": d.message },
                        "severity": d.severity.to_string(),
                        "locations": [{
                            "physicalLocation": {
                                "artifactLocation": {
                                    "uri": d.file.to_string_lossy()
                                },
                                "region": {
                                    "startLine": d.range.start.line + 1,
                                    "startColumn": d.range.start.column + 1,
                                    "endLine": d.range.end.line + 1,
                                    "endColumn": d.range.end.column + 1,
                                }
                            }
                        }]
                    })).collect::<Vec<_>>()
                }]
            });
            Ok(serde_json::to_string_pretty(&json)?)
        }
        OutputFormat::Sarif => {
            // SARIF format
            let sarif = json!({
                "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
                "version": "2.1.0",
                "runs": [{
                    "tool": {
                        "driver": {
                            "name": "bonsai-lint",
                            "informationUri": "https://bonsai.ai/lint"
                        }
                    },
                    "results": diagnostics.iter().map(|d| json!({
                        "ruleId": d.rule_id,
                        "message": { "text": d.message },
                        "level": d.severity.to_string(),
                        "locations": [{
                            "physicalLocation": {
                                "artifactLocation": {
                                    "uri": d.file.to_string_lossy()
                                },
                                "region": {
                                    "startLine": d.range.start.line + 1
                                }
                            }
                        }]
                    })).collect::<Vec<_>>()
                }]
            });
            Ok(serde_json::to_string_pretty(&sarif)?)
        }
        OutputFormat::Html => {
            let mut html = String::from(r#"<!DOCTYPE html>
<html>
<head>
<title>Lint Results</title>
<style>
table { border-collapse: collapse; width: 100%; }
th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
th { background-color: #f2f2f2; }
.error { color: red; }
.warning { color: orange; }
.hint { color: blue; }
</style>
</head>
<body>
<h1>Lint Results</h1>
<table>
<tr><th>Rule</th><th>Severity</th><th>File</th><th>Line</th><th>Message</th></tr>
"#);

            for diag in diagnostics {
                let severity_class = diag.severity.to_string();
                html.push_str(&format!(
                    r#"<tr><td>{}</td><td class="{}">{}</td><td>{}</td><td>{}</td><td>{}</td></tr>"#,
                    diag.rule_id,
                    severity_class,
                    diag.severity,
                    diag.file.to_string_lossy(),
                    diag.range.start.line + 1,
                    diag.message
                ));
            }

            html.push_str("</table></body></html>");
            Ok(html)
        }
        OutputFormat::Terminal => {
            let mut output = String::new();
            for diag in diagnostics {
                output.push_str(&format!(
                    "{}:{}:{} [{}] {}: {}\n",
                    diag.file.to_string_lossy(),
                    diag.range.start.line + 1,
                    diag.range.start.column + 1,
                    diag.severity,
                    diag.rule_id,
                    diag.message
                ));
            }
            Ok(output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Range, Position, Severity};
    use std::path::PathBuf;

    #[test]
    fn test_format_diagnostics_json() -> Result<()> {
        let range = Range::new(
            Position { line: 0, column: 0 },
            Position { line: 0, column: 10 },
            0,
            10,
        );
        let diag = Diagnostic::new(
            "test",
            "Test message",
            Severity::Warning,
            PathBuf::from("test.rs"),
            range,
        );

        let output = format_diagnostics(&[diag], OutputFormat::Json)?;
        assert!(output.contains("test"));
        Ok(())
    }

    #[test]
    fn test_format_diagnostics_terminal() -> Result<()> {
        let range = Range::new(
            Position { line: 5, column: 10 },
            Position { line: 5, column: 20 },
            50,
            60,
        );
        let diag = Diagnostic::new(
            "rule1",
            "Found issue",
            Severity::Error,
            PathBuf::from("main.rs"),
            range,
        );

        let output = format_diagnostics(&[diag], OutputFormat::Terminal)?;
        assert!(output.contains("main.rs:6:11"));
        assert!(output.contains("rule1"));
        assert!(output.contains("Found issue"));
        Ok(())
    }
}
