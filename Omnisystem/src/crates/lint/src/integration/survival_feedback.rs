/// Survival System Integration
/// Correlate crashes with lint warnings for closed-loop learning

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashReport {
    pub crash_id: String,
    pub stack_trace: String,
    pub timestamp: i64,
    pub version: String,
    pub error_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub function: String,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurvivalMetric {
    pub rule_id: String,
    pub correlation_strength: f32,
    pub false_positives: usize,
    pub true_positives: usize,
    pub ignored_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashCorrelation {
    pub rule_id: String,
    pub crash_id: String,
    pub timestamp: i64,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct LintWarningRecord {
    pub rule_id: String,
    pub file: String,
    pub line: usize,
    pub severity: String,
}

pub struct SurvivalFeedbackBridge {
    correlations: Arc<RwLock<Vec<CrashCorrelation>>>,
    metrics: Arc<RwLock<HashMap<String, SurvivalMetric>>>,
    lint_warnings: Arc<RwLock<Vec<LintWarningRecord>>>,
}

impl SurvivalFeedbackBridge {
    pub async fn new() -> Result<Self> {
        tracing::info!("Initializing Survival feedback bridge");

        Ok(Self {
            correlations: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(HashMap::new())),
            lint_warnings: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Process a crash report and correlate with lint warnings
    pub async fn on_crash(&self, crash_report: &CrashReport) -> Result<()> {
        tracing::warn!("Processing crash report: {}", crash_report.crash_id);

        let frames = Self::parse_stack_frames(&crash_report.stack_trace);

        for frame in &frames {
            if let Some(warning) = self.find_lint_warning_for(&frame).await {
                tracing::info!(
                    "Found lint warning correlation for crash in {}: rule_id={}",
                    frame.function, warning.rule_id
                );

                self.escalate_severity(&warning).await?;
                self.record_correlation(&warning.rule_id, crash_report, &frame)
                    .await?;
            }
        }

        Ok(())
    }

    /// Parse stack trace into individual frames
    fn parse_stack_frames(stack_trace: &str) -> Vec<StackFrame> {
        let mut frames = Vec::new();

        for line in stack_trace.lines() {
            if let Some(frame) = Self::parse_frame_line(line) {
                frames.push(frame);
            }
        }

        frames
    }

    fn parse_frame_line(line: &str) -> Option<StackFrame> {
        if let Some(at_pos) = line.find("at ") {
            let rest = &line[at_pos + 3..];
            if let Some(space_pos) = rest.find(' ') {
                let function = rest[..space_pos].to_string();
                let location = &rest[space_pos..];

                if let Some(colon_pos) = location.rfind(':') {
                    if let Ok(line_num) = location[colon_pos + 1..].trim().parse::<usize>() {
                        let file = location[..colon_pos].trim().to_string();
                        return Some(StackFrame {
                            function,
                            file,
                            line: line_num,
                        });
                    }
                }
            }
        }

        None
    }

    async fn find_lint_warning_for(&self, frame: &StackFrame) -> Option<LintWarningRecord> {
        let warnings = self.lint_warnings.read().await;
        warnings
            .iter()
            .find(|w| w.file == frame.file && (w.line as i32 - frame.line as i32).abs() <= 5)
            .cloned()
    }

    async fn escalate_severity(&self, warning: &LintWarningRecord) -> Result<()> {
        tracing::info!("Escalating severity for rule: {}", warning.rule_id);

        let mut metrics = self.metrics.write().await;
        if let Some(metric) = metrics.get_mut(&warning.rule_id) {
            metric.correlation_strength = (metric.correlation_strength + 0.1).min(1.0);
            metric.true_positives += 1;
        }

        Ok(())
    }

    async fn record_correlation(
        &self,
        rule_id: &str,
        crash_report: &CrashReport,
        frame: &StackFrame,
    ) -> Result<()> {
        tracing::debug!("Recording crash correlation for rule: {}", rule_id);

        let correlation = CrashCorrelation {
            rule_id: rule_id.to_string(),
            crash_id: crash_report.crash_id.clone(),
            timestamp: crash_report.timestamp,
            file: frame.file.clone(),
            line: frame.line,
        };

        let mut correlations = self.correlations.write().await;
        correlations.push(correlation);

        Ok(())
    }

    /// Get metrics on rule-crash correlations
    pub async fn get_correlation_metrics(&self, rule_id: &str) -> Result<SurvivalMetric> {
        tracing::debug!("Fetching correlation metrics for rule: {}", rule_id);

        let correlations = self.correlations.read().await;
        let rule_correlations: Vec<_> = correlations
            .iter()
            .filter(|c| c.rule_id == rule_id)
            .collect();

        let correlation_strength = if rule_correlations.is_empty() {
            0.0
        } else {
            (rule_correlations.len() as f32 / 100.0).min(1.0)
        };

        let metrics = self.metrics.read().await;
        let metric = metrics
            .get(rule_id)
            .cloned()
            .unwrap_or_else(|| SurvivalMetric {
                rule_id: rule_id.to_string(),
                correlation_strength,
                false_positives: 0,
                true_positives: rule_correlations.len(),
                ignored_count: 0,
            });

        Ok(metric)
    }

    /// Identify high-correlation rules
    pub async fn get_high_correlation_rules(&self) -> Result<Vec<SurvivalMetric>> {
        tracing::debug!("Fetching high-correlation rules");

        let metrics = self.metrics.read().await;
        let high_correlation: Vec<_> = metrics
            .values()
            .filter(|m| m.correlation_strength > 0.7)
            .cloned()
            .collect();

        tracing::info!("Found {} high-correlation rules", high_correlation.len());
        Ok(high_correlation)
    }

    /// Submit survival metric
    pub async fn submit_metric(&self, metric: SurvivalMetric) -> Result<()> {
        tracing::info!(
            "Recording survival metric for rule: {} (correlation: {:.2})",
            metric.rule_id,
            metric.correlation_strength
        );

        let mut metrics = self.metrics.write().await;
        metrics.insert(metric.rule_id.clone(), metric);

        Ok(())
    }

    /// Register a lint warning for future correlation
    pub async fn register_lint_warning(
        &self,
        rule_id: String,
        file: String,
        line: usize,
        severity: String,
    ) -> Result<()> {
        let warning = LintWarningRecord {
            rule_id,
            file,
            line,
            severity,
        };

        let mut warnings = self.lint_warnings.write().await;
        warnings.push(warning);

        Ok(())
    }

    /// Get all correlations for a rule
    pub async fn get_rule_correlations(&self, rule_id: &str) -> Result<Vec<CrashCorrelation>> {
        let correlations = self.correlations.read().await;
        let rule_correlations = correlations
            .iter()
            .filter(|c| c.rule_id == rule_id)
            .cloned()
            .collect();

        Ok(rule_correlations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_survival_bridge_creation() {
        let bridge = SurvivalFeedbackBridge::new().await.unwrap();
        assert!(true); // Bridge created successfully
    }

    #[test]
    fn test_parse_stack_frames() {
        let trace = r#"
at my_function (src/lib.rs:42)
at another_function (src/main.rs:10)
        "#;
        let frames = SurvivalFeedbackBridge::parse_stack_frames(trace);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].line, 42);
    }

    #[test]
    fn test_parse_frame_line() {
        let line = "at my_function (src/lib.rs:42)";
        let frame = SurvivalFeedbackBridge::parse_frame_line(line);
        assert!(frame.is_some());
        let f = frame.unwrap();
        assert_eq!(f.function, "my_function");
        assert_eq!(f.line, 42);
    }

    #[tokio::test]
    async fn test_get_correlation_metrics() {
        let bridge = SurvivalFeedbackBridge::new().await.unwrap();
        let metrics = bridge.get_correlation_metrics("test-rule").await.unwrap();
        assert_eq!(metrics.rule_id, "test-rule");
    }
}
