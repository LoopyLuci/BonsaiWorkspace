/// Metrics collection and reporting for KDB synchronization.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single rule metric to be reported to KDB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMetric {
    pub rule_id: String,
    pub project_id: String,
    pub language: String,
    pub domain: String,
    pub timestamp: DateTime<Utc>,

    // Performance metrics (from Phase A ETL)
    pub true_positives: u32,
    pub false_positives: u32,
    pub dismissed_count: u32,
    pub applied_fixes: u32,
    pub fix_success_rate: f32,

    // Derived metrics
    pub confidence: f32,
    pub fp_rate: f32,
    pub tp_rate: f32,
    pub dismissal_rate: f32,

    // Anonymized metadata
    pub file_count: usize,
    pub project_size_bytes: u64,
}

impl RuleMetric {
    /// Create a RuleMetric from Phase A's RuleConfidenceMetrics.
    pub fn from_etl_metrics(
        rule_id: String,
        project_id: String,
        language: String,
        domain: String,
        etl_metrics: &crate::etl::RuleConfidenceMetrics,
        file_count: usize,
        project_size_bytes: u64,
    ) -> Self {
        let total = (etl_metrics.true_positives + etl_metrics.false_positives + etl_metrics.dismissed_count).max(1);

        let tp_rate = etl_metrics.true_positives as f32 / total as f32;
        let fp_rate = etl_metrics.false_positives as f32 / total as f32;
        let dismissal_rate = etl_metrics.dismissed_count as f32 / total as f32;

        Self {
            rule_id,
            project_id,
            language,
            domain,
            timestamp: Utc::now(),
            true_positives: etl_metrics.true_positives,
            false_positives: etl_metrics.false_positives,
            dismissed_count: etl_metrics.dismissed_count,
            applied_fixes: etl_metrics.applied_fixes,
            fix_success_rate: etl_metrics.fix_success_rate,
            confidence: etl_metrics.true_positives as f32 / total as f32,
            fp_rate,
            tp_rate,
            dismissal_rate,
            file_count,
            project_size_bytes,
        }
    }
}

/// Collects metrics from Phase A and prepares them for KDB upload.
pub struct MetricsCollector {
    metrics: HashMap<String, Vec<RuleMetric>>,
    project_id: String,
    language: String,
    domain: String,
}

impl MetricsCollector {
    pub fn new(project_id: String, language: String, domain: String) -> Self {
        Self {
            metrics: HashMap::new(),
            project_id,
            language,
            domain,
        }
    }

    /// Add a rule metric.
    pub fn add_metric(&mut self, metric: RuleMetric) {
        self.metrics
            .entry(metric.rule_id.clone())
            .or_insert_with(Vec::new)
            .push(metric);
    }

    /// Get all collected metrics.
    pub fn metrics(&self) -> Vec<RuleMetric> {
        self.metrics
            .values()
            .flat_map(|v| v.clone())
            .collect()
    }

    /// Get metrics for a specific rule.
    pub fn metrics_for_rule(&self, rule_id: &str) -> Option<&Vec<RuleMetric>> {
        self.metrics.get(rule_id)
    }

    /// Get summary statistics.
    pub fn summary(&self) -> MetricsSummary {
        let all_metrics = self.metrics();

        let rule_count = self.metrics.len();
        let total_metrics = all_metrics.len();

        let avg_confidence =
            all_metrics.iter().map(|m| m.confidence).sum::<f32>() / all_metrics.len() as f32;
        let avg_fp_rate = all_metrics.iter().map(|m| m.fp_rate).sum::<f32>() / all_metrics.len() as f32;

        MetricsSummary {
            rule_count,
            total_metrics,
            avg_confidence,
            avg_fp_rate,
            project_id: self.project_id.clone(),
            language: self.language.clone(),
            domain: self.domain.clone(),
        }
    }

    /// Clear all metrics.
    pub fn clear(&mut self) {
        self.metrics.clear();
    }
}

/// Summary of collected metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub rule_count: usize,
    pub total_metrics: usize,
    pub avg_confidence: f32,
    pub avg_fp_rate: f32,
    pub project_id: String,
    pub language: String,
    pub domain: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_metric_creation() {
        let metric = RuleMetric {
            rule_id: "unused-import".to_string(),
            project_id: "proj-1".to_string(),
            language: "rust".to_string(),
            domain: "web".to_string(),
            timestamp: Utc::now(),
            true_positives: 100,
            false_positives: 10,
            dismissed_count: 5,
            applied_fixes: 100,
            fix_success_rate: 0.95,
            confidence: 0.90,
            fp_rate: 0.08,
            tp_rate: 0.90,
            dismissal_rate: 0.05,
            file_count: 1000,
            project_size_bytes: 50_000_000,
        };

        assert_eq!(metric.rule_id, "unused-import");
        assert_eq!(metric.true_positives, 100);
    }

    #[test]
    fn test_metrics_collector() {
        let mut collector = MetricsCollector::new(
            "proj-1".to_string(),
            "rust".to_string(),
            "web".to_string(),
        );

        let metric = RuleMetric {
            rule_id: "unused-import".to_string(),
            project_id: "proj-1".to_string(),
            language: "rust".to_string(),
            domain: "web".to_string(),
            timestamp: Utc::now(),
            true_positives: 100,
            false_positives: 10,
            dismissed_count: 5,
            applied_fixes: 100,
            fix_success_rate: 0.95,
            confidence: 0.90,
            fp_rate: 0.08,
            tp_rate: 0.90,
            dismissal_rate: 0.05,
            file_count: 1000,
            project_size_bytes: 50_000_000,
        };

        collector.add_metric(metric);

        let summary = collector.summary();
        assert_eq!(summary.rule_count, 1);
        assert!(summary.avg_confidence > 0.0);
    }
}
