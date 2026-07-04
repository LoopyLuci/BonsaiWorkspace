//! AI-optional advisor system for runtime optimization and selection

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Runtime recommendation with confidence score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeRecommendation {
    pub language: String,
    pub version: String,
    pub confidence: f64, // 0.0-1.0
    pub reason: String,
}

/// Performance metrics for runtime evaluation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceMetrics {
    pub execution_time_ms: u64,
    pub memory_usage_mb: u64,
    pub compatibility_score: f64, // 0.0-1.0
    pub stability_score: f64,     // 0.0-1.0
}

/// Advisor for runtime selection based on performance history
#[derive(Debug, Clone)]
pub struct RuntimeAdvisor {
    metrics_cache: HashMap<String, PerformanceMetrics>,
    recommendations_cache: HashMap<String, RuntimeRecommendation>,
}

impl RuntimeAdvisor {
    pub fn new() -> Self {
        Self {
            metrics_cache: HashMap::new(),
            recommendations_cache: HashMap::new(),
        }
    }

    /// Record performance metrics for a runtime
    pub fn record_metrics(&mut self, runtime_spec: &str, metrics: PerformanceMetrics) {
        self.metrics_cache.insert(runtime_spec.to_string(), metrics);
    }

    /// Get stored metrics for a runtime
    pub fn get_metrics(&self, runtime_spec: &str) -> Option<&PerformanceMetrics> {
        self.metrics_cache.get(runtime_spec)
    }

    /// Recommend best runtime for a language based on historical performance
    pub fn recommend_runtime(
        &self,
        language: &str,
        available_versions: &[&str],
    ) -> Option<RuntimeRecommendation> {
        if available_versions.is_empty() {
            return None;
        }

        // Sort by compatibility + stability scores
        let mut scored_versions: Vec<_> = available_versions
            .iter()
            .filter_map(|v| {
                let spec = format!("{}@{}", language, v);
                self.metrics_cache.get(&spec).map(|metrics| {
                    let score = (metrics.compatibility_score + metrics.stability_score) / 2.0;
                    (v, score, metrics)
                })
            })
            .collect();

        if scored_versions.is_empty() {
            // Default to latest version if no metrics available
            let latest = available_versions.last().unwrap();
            return Some(RuntimeRecommendation {
                language: language.to_string(),
                version: latest.to_string(),
                confidence: 0.5, // Low confidence, no data
                reason: "No historical data; using latest stable version".to_string(),
            });
        }

        scored_versions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let (best_version, score, _) = scored_versions[0];
        Some(RuntimeRecommendation {
            language: language.to_string(),
            version: best_version.to_string(),
            confidence: score.max(0.5).min(1.0),
            reason: format!(
                "Selected based on historical compatibility ({:.1}%) and stability ({:.1}%)",
                score * 100.0,
                score * 100.0
            ),
        })
    }

    /// Predict performance for a runtime based on similar workloads
    pub fn predict_performance(&self, language: &str, version: &str) -> PerformanceMetrics {
        let spec = format!("{}@{}", language, version);

        if let Some(metrics) = self.metrics_cache.get(&spec) {
            return metrics.clone();
        }

        // Default prediction if no data
        PerformanceMetrics {
            execution_time_ms: 1000, // Assume ~1 second
            memory_usage_mb: 256,    // Assume ~256 MB
            compatibility_score: 0.85,
            stability_score: 0.85,
        }
    }

    /// Get all recorded metrics
    pub fn all_metrics(&self) -> &HashMap<String, PerformanceMetrics> {
        &self.metrics_cache
    }

    /// Clear cache (for testing or resetting)
    pub fn clear(&mut self) {
        self.metrics_cache.clear();
        self.recommendations_cache.clear();
    }
}

impl Default for RuntimeAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advisor_creation() {
        let advisor = RuntimeAdvisor::new();
        assert_eq!(advisor.all_metrics().len(), 0);
    }

    #[test]
    fn test_record_and_retrieve_metrics() {
        let mut advisor = RuntimeAdvisor::new();
        let metrics = PerformanceMetrics {
            execution_time_ms: 250,
            memory_usage_mb: 256,
            compatibility_score: 0.95,
            stability_score: 0.98,
        };

        advisor.record_metrics("python@3.12.4", metrics.clone());
        assert!(advisor.get_metrics("python@3.12.4").is_some());
    }

    #[test]
    fn test_runtime_recommendation() {
        let mut advisor = RuntimeAdvisor::new();

        // Record metrics for multiple versions
        advisor.record_metrics(
            "python@3.11.0",
            PerformanceMetrics {
                execution_time_ms: 300,
                memory_usage_mb: 256,
                compatibility_score: 0.90,
                stability_score: 0.90,
            },
        );

        advisor.record_metrics(
            "python@3.12.0",
            PerformanceMetrics {
                execution_time_ms: 250,
                memory_usage_mb: 256,
                compatibility_score: 0.95,
                stability_score: 0.98,
            },
        );

        let recommendation = advisor
            .recommend_runtime("python", &["3.11.0", "3.12.0"])
            .unwrap();

        assert_eq!(recommendation.language, "python");
        assert_eq!(recommendation.version, "3.12.0");
        assert!(recommendation.confidence > 0.8);
    }

    #[test]
    fn test_performance_prediction() {
        let advisor = RuntimeAdvisor::new();
        let prediction = advisor.predict_performance("unknown", "1.0.0");
        assert_eq!(prediction.execution_time_ms, 1000);
        assert_eq!(prediction.memory_usage_mb, 256);
    }
}
