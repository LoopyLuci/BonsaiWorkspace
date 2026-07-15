/// Rule metrics aggregation across projects and domains.
/// Computes consensus metrics, identifies variants, and ranks rules.

use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Aggregated metrics for a rule across all projects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    pub rule_id: String,

    // Distribution statistics
    pub confidence_mean: f32,
    pub confidence_std: f32,
    pub confidence_min: f32,
    pub confidence_max: f32,

    // Effectiveness metrics
    pub fp_rate_mean: f32,
    pub tp_rate_mean: f32,
    pub dismissal_rate_mean: f32,

    // Coverage
    pub project_count: usize,
    pub language_distribution: HashMap<String, usize>,
    pub domain_distribution: HashMap<String, usize>,

    // Variants (rule performs differently in different domains)
    pub variants: Vec<RuleVariant>,

    // Recommendation
    pub recommended_severity: String,
    pub consensus_score: f32,  // 0-1: how much agreement across projects
}

/// Variant of a rule in a specific domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleVariant {
    pub domain: String,
    pub language: Option<String>,
    pub confidence_mean: f32,
    pub fp_rate: f32,
    pub project_count: usize,
    pub recommended_severity: String,
}

/// Aggregates rule metrics from multiple sources.
pub struct RuleMetricsAggregator {
    metrics: Arc<DashMap<String, Vec<ProjectMetrics>>>,
}

#[derive(Debug, Clone)]
pub struct ProjectMetrics {
    pub project_id: String,
    pub language: String,
    pub domain: String,
    pub confidence: f32,
    pub fp_rate: f32,
    pub tp_rate: f32,
    pub dismissal_rate: f32,
    pub project_size: usize,
}

impl RuleMetricsAggregator {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(DashMap::new()),
        }
    }

    /// Add metrics from a project for a specific rule.
    pub fn add_project_metrics(&self, rule_id: String, metrics: ProjectMetrics) -> Result<()> {
        self.metrics
            .entry(rule_id.clone())
            .or_insert_with(Vec::new)
            .push(metrics);

        tracing::debug!("Added metrics for rule: {}", rule_id);
        Ok(())
    }

    /// Get aggregated metrics for a rule.
    pub async fn get_aggregated_metrics(&self, rule_id: &str) -> Result<Option<AggregatedMetrics>> {
        let projects = match self.metrics.get(rule_id) {
            Some(entry) => entry.clone(),
            None => return Ok(None),
        };

        if projects.is_empty() {
            return Ok(None);
        }

        // Compute statistics
        let n = projects.len() as f32;

        let confidence_values: Vec<f32> = projects.iter().map(|m| m.confidence).collect();
        let fp_values: Vec<f32> = projects.iter().map(|m| m.fp_rate).collect();

        let confidence_mean = confidence_values.iter().sum::<f32>() / n;
        let confidence_std = Self::std_dev(&confidence_values, confidence_mean);

        let fp_rate_mean = fp_values.iter().sum::<f32>() / n;
        let tp_rate_mean = projects.iter().map(|m| m.tp_rate).sum::<f32>() / n;
        let dismissal_rate_mean = projects.iter().map(|m| m.dismissal_rate).sum::<f32>() / n;

        // Language distribution
        let mut lang_dist = HashMap::new();
        for metric in &projects {
            *lang_dist.entry(metric.language.clone()).or_insert(0) += 1;
        }

        // Domain distribution
        let mut domain_dist = HashMap::new();
        for metric in &projects {
            *domain_dist.entry(metric.domain.clone()).or_insert(0) += 1;
        }

        // Identify variants (per domain)
        let variants = self.compute_variants(&projects);

        // Compute consensus score (0-1: how much agreement)
        let consensus = Self::compute_consensus(&confidence_values);

        // Recommend severity based on confidence
        let recommended_severity = Self::recommend_severity(confidence_mean);

        Ok(Some(AggregatedMetrics {
            rule_id: rule_id.to_string(),
            confidence_mean,
            confidence_std,
            confidence_min: confidence_values.iter().cloned().fold(f32::INFINITY, f32::min),
            confidence_max: confidence_values.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
            fp_rate_mean,
            tp_rate_mean,
            dismissal_rate_mean,
            project_count: projects.len(),
            language_distribution: lang_dist,
            domain_distribution: domain_dist,
            variants,
            recommended_severity,
            consensus_score: consensus,
        }))
    }

    /// Compute domain-specific variants for a rule.
    fn compute_variants(&self, projects: &[ProjectMetrics]) -> Vec<RuleVariant> {
        let mut variants_map: HashMap<String, Vec<&ProjectMetrics>> = HashMap::new();

        for metric in projects {
            variants_map
                .entry(metric.domain.clone())
                .or_insert_with(Vec::new)
                .push(metric);
        }

        variants_map
            .into_iter()
            .map(|(domain, domain_metrics)| {
                let n = domain_metrics.len() as f32;
                let confidence_mean =
                    domain_metrics.iter().map(|m| m.confidence).sum::<f32>() / n;
                let fp_rate = domain_metrics.iter().map(|m| m.fp_rate).sum::<f32>() / n;

                RuleVariant {
                    domain,
                    language: domain_metrics.first().map(|m| m.language.clone()),
                    confidence_mean,
                    fp_rate,
                    project_count: domain_metrics.len(),
                    recommended_severity: Self::recommend_severity(confidence_mean),
                }
            })
            .collect()
    }

    /// Compute consensus score (how much agreement between projects).
    fn compute_consensus(values: &[f32]) -> f32 {
        if values.is_empty() {
            return 0.0;
        }

        // Compute coefficient of variation (std / mean)
        let mean = values.iter().sum::<f32>() / values.len() as f32;
        let std = Self::std_dev(values, mean);

        // Consensus = 1 - (std / mean), clamped to 0-1
        (1.0 - (std / mean.max(0.001))).max(0.0).min(1.0)
    }

    /// Compute standard deviation.
    fn std_dev(values: &[f32], mean: f32) -> f32 {
        if values.is_empty() {
            return 0.0;
        }

        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / values.len() as f32;
        variance.sqrt()
    }

    /// Recommend severity based on confidence.
    fn recommend_severity(confidence: f32) -> String {
        match confidence {
            c if c >= 0.85 => "error".to_string(),
            c if c >= 0.70 => "warning".to_string(),
            c if c >= 0.50 => "hint".to_string(),
            c if c >= 0.30 => "note".to_string(),
            _ => "disabled".to_string(),
        }
    }

    /// Clear all metrics.
    pub fn clear(&self) {
        self.metrics.clear();
    }

    /// Get total metrics stored.
    pub fn metric_count(&self) -> usize {
        self.metrics.len()
    }
}

impl Default for RuleMetricsAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_aggregator_creation() {
        let agg = RuleMetricsAggregator::new();
        assert_eq!(agg.metric_count(), 0);
    }

    #[tokio::test]
    async fn test_add_metrics() {
        let agg = RuleMetricsAggregator::new();

        let metrics = ProjectMetrics {
            project_id: "proj-1".to_string(),
            language: "rust".to_string(),
            domain: "web".to_string(),
            confidence: 0.85,
            fp_rate: 0.05,
            tp_rate: 0.95,
            dismissal_rate: 0.02,
            project_size: 5000,
        };

        agg.add_project_metrics("unused-import".to_string(), metrics)
            .unwrap();

        assert_eq!(agg.metric_count(), 1);
    }

    #[tokio::test]
    async fn test_get_aggregated_metrics() {
        let agg = RuleMetricsAggregator::new();

        for i in 0..5 {
            let metrics = ProjectMetrics {
                project_id: format!("proj-{}", i),
                language: "rust".to_string(),
                domain: if i < 3 { "web" } else { "systems" }.to_string(),
                confidence: 0.80 + (i as f32 * 0.02),
                fp_rate: 0.05,
                tp_rate: 0.95,
                dismissal_rate: 0.02,
                project_size: 5000,
            };

            agg.add_project_metrics("unused-import".to_string(), metrics)
                .unwrap();
        }

        let aggregated = agg.get_aggregated_metrics("unused-import").await.unwrap();
        assert!(aggregated.is_some());

        let metrics = aggregated.unwrap();
        assert_eq!(metrics.project_count, 5);
        assert!(metrics.confidence_mean > 0.0);
        assert_eq!(metrics.variants.len(), 2); // web and systems
    }

    #[test]
    fn test_consensus_calculation() {
        // High consensus (all similar)
        let high_consensus_values = vec![0.90, 0.91, 0.89, 0.90];
        let consensus = RuleMetricsAggregator::compute_consensus(&high_consensus_values);
        assert!(consensus > 0.95);

        // Low consensus (very different)
        let low_consensus_values = vec![0.20, 0.90, 0.10, 0.95];
        let consensus = RuleMetricsAggregator::compute_consensus(&low_consensus_values);
        assert!(consensus < 0.5);
    }
}
