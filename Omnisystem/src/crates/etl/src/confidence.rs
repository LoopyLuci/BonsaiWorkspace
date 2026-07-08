/// Rule confidence calculation and recommendation engine
use crate::events::FeedbackEvent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfidenceMetrics {
    pub rule_id: String,
    pub true_positives: u32,        // User accepted/fixed
    pub false_positives: u32,       // User rejected
    pub dismissed_count: u32,       // User ignored
    pub applied_fixes: u32,
    pub fix_success_rate: f32,      // (0.0-1.0)
    pub last_updated: DateTime<Utc>,
}

impl RuleConfidenceMetrics {
    pub fn new(rule_id: String) -> Self {
        Self {
            rule_id,
            true_positives: 0,
            false_positives: 0,
            dismissed_count: 0,
            applied_fixes: 0,
            fix_success_rate: 1.0,
            last_updated: Utc::now(),
        }
    }
}

/// Calculates dynamic confidence scores based on historical feedback
pub struct RuleConfidenceCalculator;

impl RuleConfidenceCalculator {
    /// Aggregate feedback events into metrics per rule
    pub async fn aggregate_metrics(
        &self,
        events: &[FeedbackEvent],
    ) -> anyhow::Result<HashMap<String, RuleConfidenceMetrics>> {
        let mut metrics: HashMap<String, RuleConfidenceMetrics> = HashMap::new();

        for event in events {
            let entry = metrics
                .entry(event.rule_id.clone())
                .or_insert_with(|| RuleConfidenceMetrics::new(event.rule_id.clone()));

            match event.event_type {
                crate::events::FeedbackEventType::DiagnosticAccepted => {
                    entry.true_positives += 1;
                    entry.applied_fixes += 1;

                    if event.outcome.as_deref() == Some("success") {
                        entry.fix_success_rate = (entry.fix_success_rate * (entry.applied_fixes as f32 - 1.0)
                            + 1.0)
                            / entry.applied_fixes as f32;
                    }
                }
                crate::events::FeedbackEventType::FalsePositiveReported => {
                    entry.false_positives += 1;
                }
                crate::events::FeedbackEventType::DiagnosticDismissed => {
                    entry.dismissed_count += 1;
                }
                crate::events::FeedbackEventType::DiagnosticIgnoredThenFixed => {
                    entry.true_positives += 1;
                }
                _ => {}
            }

            entry.last_updated = Utc::now();
        }

        Ok(metrics)
    }

    /// Calculate confidence for a rule based on its metrics
    pub fn calculate_confidence(&self, metrics: &RuleConfidenceMetrics) -> anyhow::Result<f32> {
        let total_observations = (metrics.true_positives + metrics.false_positives + metrics.dismissed_count)
            .max(1) as f32;

        // Base accuracy: true positives / total
        let accuracy = metrics.true_positives as f32 / total_observations;

        // Fix success penalty: if fixes are failing, lower confidence
        let fix_penalty = if metrics.applied_fixes > 0 {
            let failure_rate = 1.0 - metrics.fix_success_rate;
            (failure_rate * 0.3).min(0.3) // Max 30% penalty
        } else {
            0.0
        };

        // Dismissal factor: many dismissals = rule is noisy
        let dismissal_factor = (metrics.dismissed_count as f32 / total_observations).min(0.5);

        let confidence = (accuracy - fix_penalty) * (1.0 - dismissal_factor * 0.5);
        Ok(confidence.clamp(0.0, 1.0))
    }

    /// Recommend action based on confidence level
    pub fn recommend_action(&self, confidence: f32) -> anyhow::Result<String> {
        let action = match confidence {
            c if c >= 0.85 => "promote_to_error",
            c if c >= 0.70 => "keep_as_warning",
            c if c >= 0.50 => "demote_to_hint",
            c if c >= 0.30 => "mark_as_experimental",
            _ => "disable",
        };
        Ok(action.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_confidence_calculation() {
        let calc = RuleConfidenceCalculator;
        let mut metrics = RuleConfidenceMetrics::new("test-rule".to_string());
        metrics.true_positives = 100;
        metrics.false_positives = 5;
        metrics.dismissed_count = 0;
        metrics.applied_fixes = 100;
        metrics.fix_success_rate = 0.95;

        let confidence = calc.calculate_confidence(&metrics).unwrap();
        assert!(confidence > 0.85);
        assert!(confidence <= 1.0);

        let action = calc.recommend_action(confidence).unwrap();
        assert_eq!(action, "promote_to_error");
    }

    #[tokio::test]
    async fn test_low_confidence() {
        let calc = RuleConfidenceCalculator;
        let mut metrics = RuleConfidenceMetrics::new("noisy-rule".to_string());
        metrics.true_positives = 20;
        metrics.false_positives = 80;
        metrics.dismissed_count = 50;

        let confidence = calc.calculate_confidence(&metrics).unwrap();
        assert!(confidence < 0.30);

        let action = calc.recommend_action(confidence).unwrap();
        assert_eq!(action, "disable");
    }
}
