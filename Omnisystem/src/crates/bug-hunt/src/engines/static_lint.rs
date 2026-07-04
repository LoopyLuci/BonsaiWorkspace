/// Static linting for Rust: cargo check, clippy, cargo fmt, cargo deny.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::{debug, warn};
use regex::Regex;
use serde_json::json;
use std::path::Path;
use std::process::Command;

use crate::analyzer::LanguageAnalyzer;
use crate::finding::{Finding, Severity};

/// Rust static linting analyzer using cargo check, clippy, cargo fmt, cargo deny.
pub struct RustStaticLintAnalyzer {
    repo_path: std::path::PathBuf,
}

impl RustStaticLintAnalyzer {
    pub fn new(repo_path: std::path::PathBuf) -> Self {
        Self { repo_path }
    }

    /// Run cargo check and parse JSON output for errors.
    async fn run_cargo_check(&self) -> Result<Vec<Finding>> {
        debug!("Running 'cargo check' for Rust static analysis");

        let output = Command::new("cargo")
            .arg("check")
            .arg("--workspace")
            .arg("--message-format=json")
            .current_dir(&self.repo_path)
            .output()?;

        if !output.status.success() && output.stdout.is_empty() {
            warn!("cargo check failed: {}", String::from_utf8_lossy(&output.stderr));
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let findings = self.parse_cargo_json(&stdout)?;
        Ok(findings)
    }

    /// Run clippy and parse JSON output.
    async fn run_clippy(&self) -> Result<Vec<Finding>> {
        debug!("Running 'cargo clippy' for linting");

        let output = Command::new("cargo")
            .arg("clippy")
            .arg("--workspace")
            .arg("--message-format=json")
            .arg("--all-targets")
            .current_dir(&self.repo_path)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let findings = self.parse_cargo_json(&stdout)?;
        Ok(findings)
    }

    /// Run cargo fmt --check.
    async fn run_cargo_fmt(&self) -> Result<Vec<Finding>> {
        debug!("Running 'cargo fmt --check' for formatting");

        let output = Command::new("cargo")
            .arg("fmt")
            .arg("--check")
            .arg("--message-format=short")
            .current_dir(&self.repo_path)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut findings = Vec::new();

        for line in stdout.lines() {
            if line.contains("would reformat") {
                // Parse: "src/main.rs: would reformat (run `rustfmt --edition 2021` to format)"
                if let Some(file_part) = line.split(':').next() {
                    let file_path = self.repo_path.join(file_part);
                    findings.push(
                        Finding::new(
                            file_path,
                            1,
                            1,
                            "rustfmt::formatting".to_string(),
                            Severity::Low,
                            "Code would be reformatted by rustfmt".to_string(),
                            "rustfmt".to_string(),
                        )
                        .with_suggestion("Run 'cargo fmt' to fix formatting".to_string())
                        .with_tags(vec!["style".to_string()]),
                    );
                }
            }
        }

        Ok(findings)
    }

    /// Parse cargo's JSON message format.
    fn parse_cargo_json(&self, json_output: &str) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        for line in json_output.lines() {
            if line.is_empty() {
                continue;
            }

            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
                // Look for "compiler-message" type
                if msg.get("reason").and_then(|r| r.as_str()) == Some("compiler-message") {
                    if let Some(message) = msg.get("message") {
                        if let Some(spans) = message.get("spans").and_then(|s| s.as_array()) {
                            if let Some(span) = spans.first() {
                                if let (
                                    Some(file_name),
                                    Some(line_start),
                                    Some(line_end),
                                    Some(col_start),
                                    Some(col_end),
                                ) = (
                                    span.get("file_name").and_then(|f| f.as_str()),
                                    span.get("line_start").and_then(|l| l.as_u64()),
                                    span.get("line_end").and_then(|l| l.as_u64()),
                                    span.get("column_start").and_then(|c| c.as_u64()),
                                    span.get("column_end").and_then(|c| c.as_u64()),
                                ) {
                                    let level = message
                                        .get("level")
                                        .and_then(|l| l.as_str())
                                        .unwrap_or("warning");

                                    let severity = match level {
                                        "error" => Severity::High,
                                        "warning" => Severity::Medium,
                                        "note" => Severity::Low,
                                        _ => Severity::Info,
                                    };

                                    let msg_text = message
                                        .get("message")
                                        .and_then(|m| m.as_str())
                                        .unwrap_or("Unknown issue");

                                    let code = message
                                        .get("code")
                                        .and_then(|c| c.get("code"))
                                        .and_then(|c| c.as_str())
                                        .unwrap_or("rust");

                                    findings.push(
                                        Finding::new(
                                            std::path::PathBuf::from(file_name),
                                            line_start as usize,
                                            line_end as usize,
                                            format!("rust::{}", code),
                                            severity,
                                            msg_text.to_string(),
                                            "rustc/clippy".to_string(),
                                        )
                                        .with_columns(col_start as usize, col_end as usize),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(findings)
    }
}

#[async_trait]
impl LanguageAnalyzer for RustStaticLintAnalyzer {
    fn name(&self) -> &str {
        "rust_static_lint"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec![".rs", ".toml"]
    }

    async fn analyze_file(&self, _file_path: &Path) -> Result<Vec<Finding>> {
        // For Rust, we analyze the whole workspace, not individual files
        self.analyze_repo(&self.repo_path).await
    }

    async fn analyze_repo(&self, _repo_path: &Path) -> Result<Vec<Finding>> {
        let mut all_findings = Vec::new();

        // Run all checks in parallel (they're independent)
        let check_results = tokio::join!(
            self.run_cargo_check(),
            self.run_clippy(),
            self.run_cargo_fmt()
        );

        all_findings.extend(check_results.0?);
        all_findings.extend(check_results.1?);
        all_findings.extend(check_results.2?);

        // Deduplicate findings by (file, line, rule_id)
        all_findings.sort_by_key(|f| {
            (
                f.file_path.clone(),
                f.line_start,
                f.rule_id.clone(),
            )
        });
        all_findings.dedup_by_key(|f| {
            (
                f.file_path.clone(),
                f.line_start,
                f.rule_id.clone(),
            )
        });

        Ok(all_findings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cargo_json() {
        let analyzer = RustStaticLintAnalyzer::new(std::path::PathBuf::from("."));
        let json = r#"{"reason":"compiler-message","message":{"level":"warning","message":"unused variable","code":{"code":"unused_variables"},"spans":[{"file_name":"src/main.rs","line_start":5,"line_end":5,"column_start":5,"column_end":6}]}}"#;
        
        let findings = analyzer.parse_cargo_json(json).unwrap();
        assert!(!findings.is_empty());
    }
}
