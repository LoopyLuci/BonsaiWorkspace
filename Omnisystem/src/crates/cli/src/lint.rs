/// Linting command handlers for Omnisystem CLI
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintConfig {
    pub confidence_threshold: f32,
    pub enabled_rules: Vec<String>,
    pub disabled_rules: Vec<String>,
    pub output_format: OutputFormat,
    pub parallel: bool,
    pub cache_results: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    Text,
    Json,
    Html,
    Csv,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintResult {
    pub file_path: PathBuf,
    pub rule_id: String,
    pub severity: String,
    pub message: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintReport {
    pub file_count: usize,
    pub issue_count: usize,
    pub errors: usize,
    pub warnings: usize,
    pub hints: usize,
    pub results: Vec<LintResult>,
    pub duration_ms: u128,
}

pub struct LintCommand {
    config: Arc<RwLock<LintConfig>>,
    results_cache: Arc<RwLock<HashMap<String, LintReport>>>,
}

impl LintCommand {
    pub fn new(config: LintConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            results_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn lint_file(&self, file_path: &Path) -> Result<LintReport> {
        let start = std::time::Instant::now();
        tracing::info!("Linting file: {:?}", file_path);

        let config = self.config.read().await;
        let mut results = Vec::new();

        let file_key = file_path.to_string_lossy().to_string();

        if config.cache_results {
            let cache = self.results_cache.read().await;
            if let Some(cached) = cache.get(&file_key) {
                tracing::debug!("Using cached lint results for {:?}", file_path);
                return Ok(cached.clone());
            }
        }

        for (idx, _rule) in config.enabled_rules.iter().enumerate() {
            if results.len() >= 10 {
                break;
            }

            results.push(LintResult {
                file_path: file_path.to_path_buf(),
                rule_id: format!("rule-{}", idx),
                severity: if idx % 2 == 0 { "warning" } else { "info" }.to_string(),
                message: "Sample lint finding".to_string(),
                line: (idx + 1) as u32,
                column: 5,
            });
        }

        let duration = start.elapsed();
        let report = LintReport {
            file_count: 1,
            issue_count: results.len(),
            errors: results.iter().filter(|r| r.severity == "error").count(),
            warnings: results.iter().filter(|r| r.severity == "warning").count(),
            hints: results.iter().filter(|r| r.severity == "hint").count(),
            results,
            duration_ms: duration.as_millis(),
        };

        if config.cache_results {
            let mut cache = self.results_cache.write().await;
            cache.insert(file_key, report.clone());
        }

        tracing::info!("Lint complete: {} issues found", report.issue_count);
        Ok(report)
    }

    pub async fn lint_directory(&self, dir_path: &Path) -> Result<LintReport> {
        let start = std::time::Instant::now();
        tracing::info!("Linting directory: {:?}", dir_path);

        let config = self.config.read().await;
        let mut all_results = Vec::new();
        let mut file_count = 0;

        for _i in 0..5 {
            file_count += 1;
            all_results.push(LintResult {
                file_path: dir_path.join(format!("file{}.rs", _i)),
                rule_id: "rule-1".to_string(),
                severity: "warning".to_string(),
                message: "Sample finding".to_string(),
                line: 10,
                column: 5,
            });
        }

        let duration = start.elapsed();
        let report = LintReport {
            file_count,
            issue_count: all_results.len(),
            errors: 0,
            warnings: all_results.len(),
            hints: 0,
            results: all_results,
            duration_ms: duration.as_millis(),
        };

        tracing::info!("Directory lint complete: {} files, {} issues", file_count, report.issue_count);
        Ok(report)
    }

    pub async fn set_confidence_threshold(&self, threshold: f32) -> Result<()> {
        let mut config = self.config.write().await;
        config.confidence_threshold = threshold.clamp(0.0, 1.0);
        tracing::info!("Set confidence threshold to {}", threshold);
        Ok(())
    }

    pub async fn enable_rule(&self, rule_id: String) -> Result<()> {
        let mut config = self.config.write().await;
        if !config.enabled_rules.contains(&rule_id) {
            config.enabled_rules.push(rule_id.clone());
            config.disabled_rules.retain(|r| r != &rule_id);
        }
        tracing::info!("Enabled rule: {}", rule_id);
        Ok(())
    }

    pub async fn disable_rule(&self, rule_id: String) -> Result<()> {
        let mut config = self.config.write().await;
        if !config.disabled_rules.contains(&rule_id) {
            config.disabled_rules.push(rule_id.clone());
            config.enabled_rules.retain(|r| r != &rule_id);
        }
        tracing::info!("Disabled rule: {}", rule_id);
        Ok(())
    }

    pub async fn set_output_format(&self, format: OutputFormat) -> Result<()> {
        let mut config = self.config.write().await;
        config.output_format = format;
        tracing::info!("Set output format: {:?}", format);
        Ok(())
    }

    pub async fn enable_parallel(&self) -> Result<()> {
        let mut config = self.config.write().await;
        config.parallel = true;
        tracing::info!("Enabled parallel processing");
        Ok(())
    }

    pub async fn clear_cache(&self) -> Result<()> {
        let mut cache = self.results_cache.write().await;
        let count = cache.len();
        cache.clear();
        tracing::info!("Cleared cache: {} entries removed", count);
        Ok(())
    }

    pub async fn format_report(&self, report: &LintReport) -> Result<String> {
        let config = self.config.read().await;

        let output = match config.output_format {
            OutputFormat::Json => serde_json::to_string_pretty(report)?,
            OutputFormat::Csv => self.report_to_csv(report),
            OutputFormat::Html => self.report_to_html(report),
            OutputFormat::Text => self.report_to_text(report),
        };

        Ok(output)
    }

    fn report_to_text(&self, report: &LintReport) -> String {
        format!(
            "Lint Report\n\
             Files: {}\n\
             Issues: {}\n\
             Errors: {}\n\
             Warnings: {}\n\
             Hints: {}\n\
             Duration: {}ms",
            report.file_count, report.issue_count, report.errors, report.warnings, report.hints, report.duration_ms
        )
    }

    fn report_to_csv(&self, report: &LintReport) -> String {
        let mut csv = "file,rule,severity,message,line,column\n".to_string();
        for result in &report.results {
            csv.push_str(&format!(
                "{},{},{},{},{},{}\n",
                result.file_path.display(),
                result.rule_id,
                result.severity,
                result.message,
                result.line,
                result.column
            ));
        }
        csv
    }

    fn report_to_html(&self, report: &LintReport) -> String {
        format!(
            "<html><body><h1>Lint Report</h1>\
             <p>Files: {}</p>\
             <p>Issues: {}</p>\
             <table border='1'><tr><th>File</th><th>Rule</th><th>Severity</th></tr>",
            report.file_count, report.issue_count
        )
    }

    pub async fn get_enabled_rules(&self) -> Result<Vec<String>> {
        let config = self.config.read().await;
        Ok(config.enabled_rules.clone())
    }

    pub async fn get_disabled_rules(&self) -> Result<Vec<String>> {
        let config = self.config.read().await;
        Ok(config.disabled_rules.clone())
    }
}

impl Default for LintCommand {
    fn default() -> Self {
        let config = LintConfig {
            confidence_threshold: 0.7,
            enabled_rules: vec!["rule-1".to_string(), "rule-2".to_string()],
            disabled_rules: Vec::new(),
            output_format: OutputFormat::Text,
            parallel: false,
            cache_results: true,
        };
        Self::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lint_file() {
        let cmd = LintCommand::default();
        let result = cmd.lint_file(Path::new("test.rs")).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_enable_disable_rules() {
        let cmd = LintCommand::default();
        let _ = cmd.enable_rule("test-rule".to_string()).await;
        let enabled = cmd.get_enabled_rules().await.unwrap();
        assert!(enabled.contains(&"test-rule".to_string()));
    }

    #[tokio::test]
    async fn test_confidence_threshold() {
        let cmd = LintCommand::default();
        let _ = cmd.set_confidence_threshold(0.9).await;
        let config = cmd.config.read().await;
        assert_eq!(config.confidence_threshold, 0.9);
    }

    #[tokio::test]
    async fn test_output_formats() {
        let cmd = LintCommand::default();
        for format in &[OutputFormat::Json, OutputFormat::Csv, OutputFormat::Html, OutputFormat::Text] {
            let result = cmd.set_output_format(*format).await;
            assert!(result.is_ok());
        }
    }
}
