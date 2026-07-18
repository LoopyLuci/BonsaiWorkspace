/// Rule refinement engine - proposes mutations for low-confidence rules
use crate::confidence::RuleConfidenceMetrics;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMutationProposal {
    pub proposal_id: String,
    pub rule_id: String,
    pub original_pattern: String,
    pub mutated_pattern: String,
    pub expected_improvement: f32,
    pub false_positive_examples: Vec<String>,
    pub true_positive_examples: Vec<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Proposes refinements to improve low-confidence rules
pub struct RuleRefiner;

impl RuleRefiner {
    pub fn new() -> Self {
        Self
    }

    /// Propose refinements for rules in the 0.50-0.70 confidence range
    pub async fn propose_refinements(
        &self,
        metrics: &HashMap<String, RuleConfidenceMetrics>,
    ) -> anyhow::Result<Vec<RuleMutationProposal>> {
        let mut proposals = Vec::new();

        for (rule_id, metric) in metrics {
            // Only refine rules in the "fixable" confidence range
            if metric.true_positives + metric.false_positives + metric.dismissed_count < 10 {
                continue; // Not enough data
            }

            let false_positive_rate =
                metric.false_positives as f32 / (metric.true_positives + metric.false_positives).max(1) as f32;

            // Only propose if false positive rate > 10%
            if false_positive_rate > 0.10 {
                let proposal = RuleMutationProposal {
                    proposal_id: uuid::Uuid::new_v4().to_string(),
                    rule_id: rule_id.clone(),
                    original_pattern: format!("pattern_for_{}", rule_id), // Placeholder
                    mutated_pattern: format!("refined_pattern_for_{}", rule_id), // Placeholder
                    expected_improvement: (false_positive_rate * 0.5).min(0.2), // Estimate 50% FP reduction
                    false_positive_examples: vec![],                              // Would populate from TDL
                    true_positive_examples: vec![],                              // Would populate from TDL
                    timestamp: Utc::now(),
                };

                proposals.push(proposal);
            }
        }

        tracing::info!("Proposed {} rule refinements", proposals.len());
        Ok(proposals)
    }

    /// Evaluate a mutation against test set
    pub async fn evaluate_mutation(&self, _proposal: &RuleMutationProposal) -> anyhow::Result<f32> {
        // TODO: Implement actual mutation evaluation
        // This would test the mutated pattern against a held-out test set
        Ok(0.75) // Placeholder
    }
}

impl Default for RuleRefiner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_propose_refinements() {
        let refiner = RuleRefiner::new();
        let mut metrics = HashMap::new();

        let mut metric = RuleConfidenceMetrics::new("noisy-rule".to_string());
        metric.true_positives = 60;
        metric.false_positives = 30;
        metric.dismissed_count = 10;
        metrics.insert("noisy-rule".to_string(), metric);

        let proposals = refiner.propose_refinements(&metrics).await.unwrap();
        assert!(!proposals.is_empty());
        assert_eq!(proposals[0].rule_id, "noisy-rule");
    }
}
