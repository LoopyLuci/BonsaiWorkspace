//! Weakness prediction and AI-guided analysis.
//!
//! Analyzes chaos test results to identify system weaknesses and recommend
//! additional hardening or testing strategies.

use crate::suite_executor::ChaosTestResults;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// Predicted weakness in system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaknessPrediction {
    /// Name of the weakness.
    pub name: String,
    /// Component affected.
    pub component: String,
    /// Severity (1-10).
    pub severity: u8,
    /// Confidence level (0-100%).
    pub confidence: u8,
    /// Detailed description.
    pub description: String,
    /// Fault types that trigger this weakness.
    pub triggering_faults: Vec<String>,
    /// Recommended hardening strategies.
    pub recommendations: Vec<String>,
    /// Evidence from test results.
    pub evidence: Vec<String>,
}

impl WeaknessPrediction {
    /// Create new weakness prediction.
    pub fn new(name: String, component: String, severity: u8, confidence: u8) -> Self {
        Self {
            name,
            component,
            severity,
            confidence,
            description: String::new(),
            triggering_faults: Vec::new(),
            recommendations: Vec::new(),
            evidence: Vec::new(),
        }
    }

    /// Add description.
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }

    /// Add triggering fault.
    pub fn with_triggering_fault(mut self, fault: String) -> Self {
        self.triggering_faults.push(fault);
        self
    }

    /// Add recommendation.
    pub fn with_recommendation(mut self, rec: String) -> Self {
        self.recommendations.push(rec);
        self
    }

    /// Add evidence.
    pub fn with_evidence(mut self, ev: String) -> Self {
        self.evidence.push(ev);
        self
    }
}

/// Weakness analysis report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaknessAnalysis {
    /// Discovered weaknesses.
    pub weaknesses: Vec<WeaknessPrediction>,
    /// Fault type effectiveness (which faults expose most issues).
    pub fault_effectiveness: HashMap<String, u32>,
    /// Component vulnerability scores.
    pub component_scores: HashMap<String, f64>,
    /// Overall system resilience score (0-100).
    pub overall_resilience_score: u32,
    /// Critical path vulnerabilities.
    pub critical_path_issues: Vec<String>,
}

impl Default for WeaknessAnalysis {
    fn default() -> Self {
        Self {
            weaknesses: Vec::new(),
            fault_effectiveness: HashMap::new(),
            component_scores: HashMap::new(),
            overall_resilience_score: 100,
            critical_path_issues: Vec::new(),
        }
    }
}

/// Weakness predictor analyzer.
pub struct WeaknessPredictor;

impl WeaknessPredictor {
    /// Analyze test results for weaknesses.
    pub fn analyze(results: &ChaosTestResults) -> WeaknessAnalysis {
        let mut analysis = WeaknessAnalysis::default();

        // Analyze failure patterns
        let failures: Vec<_> = results.runs.iter().filter(|r| !r.passed).collect();
        let failure_rate = (failures.len() * 100) / results.runs.len().max(1);

        // If high failure rate, identify weaknesses
        if failure_rate > 30 {
            analysis.overall_resilience_score = (100 - failure_rate as u32).max(0);

            // Predict common weaknesses based on failure patterns
            Self::predict_common_weaknesses(&mut analysis, results);
        }

        // Identify fault effectiveness
        Self::analyze_fault_effectiveness(&mut analysis, results);

        // Identify critical path issues
        Self::identify_critical_path_issues(&mut analysis, results);

        info!(
            "Weakness analysis complete: {} weaknesses identified, resilience score = {}",
            analysis.weaknesses.len(),
            analysis.overall_resilience_score
        );

        analysis
    }

    fn predict_common_weaknesses(analysis: &mut WeaknessAnalysis, results: &ChaosTestResults) {
        // Recovery time weakness
        let slow_recoveries: Vec<_> = results
            .runs
            .iter()
            .flat_map(|r| {
                r.recovery_metrics
                    .iter()
                    .filter(|m| m.recovery_time_ms > 10000)
            })
            .collect();

        if slow_recoveries.len() > 0 {
            let weakness = WeaknessPrediction::new(
                "Slow Recovery Time".to_string(),
                "Recovery Path".to_string(),
                7,
                85,
            )
            .with_description(
                "System takes longer than 10 seconds to recover from certain faults. \
                 This could violate SLAs and cascade into other failures."
                    .to_string(),
            )
            .with_triggering_fault("Storage Corruption".to_string())
            .with_triggering_fault("Network Partition".to_string())
            .with_recommendation("Implement recovery parallelization".to_string())
            .with_recommendation("Add recovery state checkpointing".to_string())
            .with_recommendation("Implement partial recovery (degraded mode)".to_string())
            .with_evidence(format!("{} faults showed slow recovery", slow_recoveries.len()));

            analysis.weaknesses.push(weakness);
        }

        // Detection lag weakness
        let slow_detection: Vec<_> = results
            .runs
            .iter()
            .flat_map(|r| {
                r.recovery_metrics
                    .iter()
                    .filter(|m| m.detection_time_ms > 5000)
            })
            .collect();

        if slow_detection.len() > 0 {
            let weakness = WeaknessPrediction::new(
                "Slow Fault Detection".to_string(),
                "Monitoring".to_string(),
                6,
                80,
            )
            .with_description(
                "Faults are not detected quickly enough, leading to prolonged \
                 service degradation before recovery begins."
                    .to_string(),
            )
            .with_recommendation("Implement active health checks".to_string())
            .with_recommendation("Reduce health check interval".to_string())
            .with_recommendation("Add anomaly detection".to_string())
            .with_evidence(format!("{} faults had delayed detection", slow_detection.len()));

            analysis.weaknesses.push(weakness);
        }

        // Data loss weakness
        let data_loss: Vec<_> = results
            .runs
            .iter()
            .flat_map(|r| {
                r.recovery_metrics
                    .iter()
                    .filter(|m| !m.data_integrity_ok)
            })
            .collect();

        if data_loss.len() > 0 {
            let weakness = WeaknessPrediction::new(
                "Data Loss Under Failures".to_string(),
                "Storage".to_string(),
                10,
                95,
            )
            .with_description(
                "Some fault scenarios result in data loss. This is a critical \
                 system weakness that must be addressed."
                    .to_string(),
            )
            .with_recommendation("Implement write-ahead logging".to_string())
            .with_recommendation("Add replica confirmation before ACK".to_string())
            .with_recommendation("Implement crash recovery verification".to_string())
            .with_evidence(format!("{} faults caused data loss", data_loss.len()));

            analysis.weaknesses.push(weakness);
            analysis.critical_path_issues.push("Data loss detected".to_string());
        }

        // Consistency violation weakness
        let consistency_issues: Vec<_> = results
            .runs
            .iter()
            .flat_map(|r| {
                r.recovery_metrics
                    .iter()
                    .filter(|m| m.consistency_violations > 0)
            })
            .collect();

        if consistency_issues.len() > 0 {
            let total_violations: u64 = consistency_issues.iter().map(|m| m.consistency_violations).sum();
            let weakness = WeaknessPrediction::new(
                "Consistency Violations".to_string(),
                "Distributed System".to_string(),
                8,
                90,
            )
            .with_description(
                "Faults cause the system to reach inconsistent states where \
                 different replicas or components have divergent views."
                    .to_string(),
            )
            .with_recommendation("Implement invariant checks".to_string())
            .with_recommendation("Add consistency verification tests".to_string())
            .with_recommendation("Implement anti-entropy mechanism".to_string())
            .with_evidence(format!("{} total consistency violations detected", total_violations));

            analysis.weaknesses.push(weakness);
            analysis.critical_path_issues.push("Consistency violations".to_string());
        }
    }

    fn analyze_fault_effectiveness(analysis: &mut WeaknessAnalysis, results: &ChaosTestResults) {
        let mut fault_counts: HashMap<String, u32> = HashMap::new();

        for run in &results.runs {
            for metric in &run.recovery_metrics {
                if !metric.successful {
                    // This fault type caused issues
                    *fault_counts
                        .entry("Recovery Failure".to_string())
                        .or_insert(0) += 1;
                }
                if metric.recovery_time_ms > 10000 {
                    *fault_counts
                        .entry("Slow Recovery".to_string())
                        .or_insert(0) += 1;
                }
            }
        }

        analysis.fault_effectiveness = fault_counts;
    }

    fn identify_critical_path_issues(
        analysis: &mut WeaknessAnalysis,
        results: &ChaosTestResults,
    ) {
        // Check if any runs failed completely
        let complete_failures = results.runs.iter().filter(|r| !r.passed).count();
        if complete_failures > results.runs.len() / 2 {
            analysis
                .critical_path_issues
                .push("More than 50% of runs failed".to_string());
            analysis.overall_resilience_score = analysis.overall_resilience_score.saturating_sub(30);
        }

        // Check for consistent patterns
        if results.runs.windows(2).any(|w| !w[0].passed && !w[1].passed) {
            analysis
                .critical_path_issues
                .push("Consecutive failures detected".to_string());
        }
    }
}

/// Generate recommendations based on analysis.
pub struct RecommendationGenerator;

impl RecommendationGenerator {
    /// Generate prioritized recommendations.
    pub fn generate(analysis: &WeaknessAnalysis) -> Vec<PrioritizedRecommendation> {
        let mut recommendations = Vec::new();

        for weakness in &analysis.weaknesses {
            for (idx, rec) in weakness.recommendations.iter().enumerate() {
                let priority = Self::calculate_priority(weakness.severity, weakness.confidence, idx);

                recommendations.push(PrioritizedRecommendation {
                    recommendation: rec.clone(),
                    weakness: weakness.name.clone(),
                    component: weakness.component.clone(),
                    priority,
                    severity: weakness.severity,
                    confidence: weakness.confidence,
                    estimated_effort: Self::estimate_effort(rec),
                });
            }
        }

        recommendations.sort_by_key(|r| std::cmp::Reverse(r.priority));
        recommendations
    }

    fn calculate_priority(severity: u8, confidence: u8, position: usize) -> u32 {
        ((severity as u32) * (confidence as u32) / 100) + (100 / (position as u32 + 1))
    }

    fn estimate_effort(recommendation: &str) -> EffortEstimate {
        match recommendation {
            _ if recommendation.contains("simple") || recommendation.contains("add") => {
                EffortEstimate::Low
            }
            _ if recommendation.contains("redesign") || recommendation.contains("architecture") => {
                EffortEstimate::High
            }
            _ => EffortEstimate::Medium,
        }
    }
}

/// Prioritized recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrioritizedRecommendation {
    pub recommendation: String,
    pub weakness: String,
    pub component: String,
    pub priority: u32,
    pub severity: u8,
    pub confidence: u8,
    pub estimated_effort: EffortEstimate,
}

/// Effort estimate for implementing a recommendation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EffortEstimate {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for EffortEstimate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EffortEstimate::Low => write!(f, "Low"),
            EffortEstimate::Medium => write!(f, "Medium"),
            EffortEstimate::High => write!(f, "High"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weakness_creation() {
        let weakness = WeaknessPrediction::new(
            "Test".to_string(),
            "Component".to_string(),
            5,
            75,
        )
        .with_description("Test weakness".to_string())
        .with_triggering_fault("Fault1".to_string());

        assert_eq!(weakness.name, "Test");
        assert_eq!(weakness.severity, 5);
        assert_eq!(weakness.confidence, 75);
        assert_eq!(weakness.triggering_faults.len(), 1);
    }

    #[test]
    fn test_weakness_analysis_default() {
        let analysis = WeaknessAnalysis::default();
        assert_eq!(analysis.weaknesses.len(), 0);
        assert_eq!(analysis.overall_resilience_score, 100);
    }

    #[test]
    fn test_recommendation_generator() {
        let mut analysis = WeaknessAnalysis::default();
        analysis.weaknesses.push(
            WeaknessPrediction::new("Test".to_string(), "Comp".to_string(), 8, 90)
                .with_recommendation("Fix this".to_string()),
        );

        let recs = RecommendationGenerator::generate(&analysis);
        assert!(!recs.is_empty());
    }
}
