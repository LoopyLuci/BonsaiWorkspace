use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Detects regressions when model changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionReport {
    pub test_name: String,
    pub current_version: String,
    pub baseline_version: String,
    pub timestamp: String,

    pub regressions_detected: Vec<RegressionEvent>,
    pub all_metrics_passed: bool,
    pub regression_severity: f32,  // 0-1: how severe the regressions are

    pub detailed_comparisons: HashMap<String, MetricComparison>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionEvent {
    pub metric_name: String,
    pub scale: u32,
    pub baseline_value: f32,
    pub current_value: f32,
    pub regression_pct: f32,
    pub threshold_pct: f32,
    pub severity: String,  // "minor", "moderate", "severe"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricComparison {
    pub metric_name: String,
    pub baseline: f32,
    pub current: f32,
    pub change_pct: f32,
    pub passed: bool,
}

pub struct RegressionDetector {
    pub regression_threshold_pct: f32,
}

impl Default for RegressionDetector {
    fn default() -> Self {
        Self {
            regression_threshold_pct: 5.0,
        }
    }
}

impl RegressionDetector {
    pub fn new(threshold_pct: f32) -> Self {
        Self {
            regression_threshold_pct: threshold_pct,
        }
    }

    /// Check for regressions in latency metrics
    pub fn check_latency_regressions(
        &self,
        baseline: &HashMap<String, f32>,
        current: &HashMap<String, f32>,
    ) -> Vec<RegressionEvent> {
        let mut regressions = Vec::new();

        for (key, baseline_val) in baseline {
            if let Some(&current_val) = current.get(key) {
                // For latency, higher is worse
                let regression_pct = ((current_val - baseline_val) / baseline_val * 100.0).max(0.0);

                if regression_pct > self.regression_threshold_pct {
                    let severity = match regression_pct {
                        x if x > 20.0 => "severe",
                        x if x > 10.0 => "moderate",
                        _ => "minor",
                    };

                    regressions.push(RegressionEvent {
                        metric_name: key.clone(),
                        scale: 0,  // Would be extracted from key
                        baseline_value: *baseline_val,
                        current_value: current_val,
                        regression_pct,
                        threshold_pct: self.regression_threshold_pct,
                        severity: severity.to_string(),
                    });
                }
            }
        }

        regressions
    }

    /// Check for regressions in quality metrics (lower is worse)
    pub fn check_quality_regressions(
        &self,
        baseline: &HashMap<String, f32>,
        current: &HashMap<String, f32>,
    ) -> Vec<RegressionEvent> {
        let mut regressions = Vec::new();

        for (key, baseline_val) in baseline {
            if let Some(&current_val) = current.get(key) {
                // For quality, lower is worse
                let regression_pct = (((baseline_val - current_val) / baseline_val) * 100.0).max(0.0);

                if regression_pct > self.regression_threshold_pct {
                    let severity = match regression_pct {
                        x if x > 20.0 => "severe",
                        x if x > 10.0 => "moderate",
                        _ => "minor",
                    };

                    regressions.push(RegressionEvent {
                        metric_name: key.clone(),
                        scale: 0,
                        baseline_value: *baseline_val,
                        current_value: current_val,
                        regression_pct,
                        threshold_pct: self.regression_threshold_pct,
                        severity: severity.to_string(),
                    });
                }
            }
        }

        regressions
    }

    /// Run comprehensive regression detection
    pub fn detect_all_regressions(
        &self,
        baseline: &HashMap<String, f32>,
        current: &HashMap<String, f32>,
    ) -> RegressionReport {
        // Categorize metrics
        let (baseline_latency, baseline_quality) = self.categorize_metrics(baseline);
        let (current_latency, current_quality) = self.categorize_metrics(current);

        let latency_regressions = self.check_latency_regressions(&baseline_latency, &current_latency);
        let quality_regressions = self.check_quality_regressions(&baseline_quality, &current_quality);

        let mut all_regressions = Vec::new();
        all_regressions.extend(latency_regressions);
        all_regressions.extend(quality_regressions);

        let all_metrics_passed = all_regressions.is_empty();

        let regression_severity = if all_regressions.is_empty() {
            0.0
        } else {
            all_regressions.iter()
                .map(|r| {
                    match r.severity.as_str() {
                        "severe" => 1.0,
                        "moderate" => 0.5,
                        _ => 0.2,
                    }
                })
                .sum::<f32>() / all_regressions.len() as f32
        };

        let detailed_comparisons = self.build_detailed_comparisons(baseline, current);

        RegressionReport {
            test_name: "comprehensive_regression_test".to_string(),
            current_version: "v0.2.0".to_string(),
            baseline_version: "v0.1.0".to_string(),
            timestamp: chrono::Local::now().to_rfc3339(),
            regressions_detected: all_regressions,
            all_metrics_passed,
            regression_severity,
            detailed_comparisons,
        }
    }

    pub fn should_rollback(&self, report: &RegressionReport) -> bool {
        // Rollback if there are severe regressions
        report.regression_severity > 0.7 ||
        report.regressions_detected.iter().any(|r| r.severity == "severe")
    }

    fn categorize_metrics(&self, metrics: &HashMap<String, f32>) -> (HashMap<String, f32>, HashMap<String, f32>) {
        let mut latency = HashMap::new();
        let mut quality = HashMap::new();

        for (key, value) in metrics {
            if key.contains("latency") || key.contains("ttft") || key.contains("tpt") {
                latency.insert(key.clone(), *value);
            } else {
                quality.insert(key.clone(), *value);
            }
        }

        (latency, quality)
    }

    fn build_detailed_comparisons(
        &self,
        baseline: &HashMap<String, f32>,
        current: &HashMap<String, f32>,
    ) -> HashMap<String, MetricComparison> {
        let mut comparisons = HashMap::new();

        for (key, baseline_val) in baseline {
            if let Some(&current_val) = current.get(key) {
                let change_pct = if baseline_val.abs() > 1e-9 {
                    ((current_val - baseline_val) / baseline_val) * 100.0
                } else {
                    0.0
                };

                let passed = change_pct.abs() <= self.regression_threshold_pct;

                comparisons.insert(
                    key.clone(),
                    MetricComparison {
                        metric_name: key.clone(),
                        baseline: *baseline_val,
                        current: current_val,
                        change_pct,
                        passed,
                    },
                );
            }
        }

        comparisons
    }
}

/// Automated rollback on regression
pub struct RollbackManager {
    pub max_rollback_age_hours: u32,
}

impl Default for RollbackManager {
    fn default() -> Self {
        Self {
            max_rollback_age_hours: 24,
        }
    }
}

impl RollbackManager {
    pub async fn perform_rollback(&self, version: &str) -> Result<(), String> {
        // In production, this would:
        // 1. Stop current inference
        // 2. Load previous model weights
        // 3. Resume inference
        // 4. Log incident

        tracing::warn!(
            "Rolling back to version {} due to regressions",
            version
        );

        Ok(())
    }

    pub fn get_last_known_good_version(&self) -> String {
        // Query from version store
        "v0.1.0".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_regression_detection() {
        let detector = RegressionDetector::new(5.0);

        let mut baseline = HashMap::new();
        baseline.insert("latency_100m".to_string(), 10.0);
        baseline.insert("latency_1b".to_string(), 20.0);

        let mut current = HashMap::new();
        current.insert("latency_100m".to_string(), 10.5);  // 5% increase - should pass
        current.insert("latency_1b".to_string(), 22.0);    // 10% increase - should fail

        let regressions = detector.check_latency_regressions(&baseline, &current);
        assert_eq!(regressions.len(), 1);
        assert!(regressions[0].metric_name.contains("1b"));
    }

    #[test]
    fn test_quality_regression_detection() {
        let detector = RegressionDetector::new(5.0);

        let mut baseline = HashMap::new();
        baseline.insert("mmlu_score".to_string(), 80.0);

        let mut current = HashMap::new();
        current.insert("mmlu_score".to_string(), 75.0);  // 6.25% decrease - should fail

        let regressions = detector.check_quality_regressions(&baseline, &current);
        assert_eq!(regressions.len(), 1);
    }

    #[test]
    fn test_rollback_decision() {
        let detector = RegressionDetector::new(5.0);
        let rollback = RollbackManager::default();

        let mut baseline = HashMap::new();
        baseline.insert("latency".to_string(), 10.0);

        let mut current = HashMap::new();
        current.insert("latency".to_string(), 25.0);  // 150% increase

        let report = detector.detect_all_regressions(&baseline, &current);
        assert!(rollback.should_rollback(&report));
    }
}

// Add chrono dependency for timestamp
use chrono;
