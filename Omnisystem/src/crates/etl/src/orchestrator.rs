//! EternalTrainingLoop orchestrator: ties feedback storage, confidence
//! calculation, rule adjustment, refinement proposals, and Universe
//! event emission together into a single cycle.
//!
//! This type was referenced by both `tests/integration_tests.rs` and
//! `src/bin/daemon.rs` in the archived source but never actually
//! defined anywhere - every other piece of the pipeline (storage,
//! confidence math, refiner, event emitter) existed and was tested in
//! isolation, just never wired into the orchestrator that ties them
//! together.

use crate::adjuster::RuleConfidenceAdjuster;
use crate::confidence::RuleConfidenceCalculator;
use crate::events::UniverseEventEmitter;
use crate::refiner::RuleRefiner;
use crate::storage::ETLStorage;
use crate::RuleConfidenceUpdate;
use chrono::Utc;
use std::sync::Arc;

/// Summary of a single ETL cycle.
#[derive(Debug, Clone)]
pub struct CycleResult {
    pub feedback_events_processed: usize,
    pub rules_analyzed: usize,
    pub confidence_updates_applied: usize,
    pub refinement_proposals: usize,
}

/// Orchestrates one full feedback -> confidence -> adjustment ->
/// refinement -> observability cycle.
pub struct EternalTrainingLoop {
    storage: Arc<ETLStorage>,
    calculator: Arc<RuleConfidenceCalculator>,
    adjuster: Arc<RuleConfidenceAdjuster>,
    refiner: Arc<RuleRefiner>,
    event_emitter: Arc<UniverseEventEmitter>,
}

impl EternalTrainingLoop {
    pub fn new(
        storage: Arc<ETLStorage>,
        calculator: Arc<RuleConfidenceCalculator>,
        adjuster: Arc<RuleConfidenceAdjuster>,
        refiner: Arc<RuleRefiner>,
        event_emitter: Arc<UniverseEventEmitter>,
    ) -> Self {
        Self {
            storage,
            calculator,
            adjuster,
            refiner,
            event_emitter,
        }
    }

    /// Run one full cycle: pull recent feedback, recompute per-rule
    /// confidence, apply adjustments, propose refinements for noisy
    /// rules, and emit observability events for all of it.
    pub async fn run_cycle(&self) -> anyhow::Result<CycleResult> {
        // No persisted cursor exists, so look back over a generous
        // window to catch all feedback not yet folded into cached
        // metrics.
        let since = Utc::now() - chrono::Duration::days(365);
        let events = self.storage.get_feedback_events_since(since).await?;

        let metrics = self.calculator.aggregate_metrics(&events).await?;

        let mut confidence_updates_applied = 0;
        for (rule_id, metric) in &metrics {
            // Compare against whatever confidence was cached from the
            // previous cycle (if any) so the update reflects real
            // movement rather than a fabricated "before" value.
            let old_confidence = match self.storage.get_metrics(rule_id).await? {
                Some(previous) => self
                    .calculator
                    .calculate_confidence(&previous)
                    .unwrap_or(1.0),
                None => 1.0,
            };

            let new_confidence = self.calculator.calculate_confidence(metric)?;
            let action = self.calculator.recommend_action(new_confidence)?;

            let update = RuleConfidenceUpdate {
                rule_id: rule_id.clone(),
                old_confidence,
                new_confidence,
                action,
                true_positives: metric.true_positives,
                false_positives: metric.false_positives,
                dismissed_count: metric.dismissed_count,
                timestamp: Utc::now(),
            };

            self.adjuster.apply_update(&update).await?;
            self.event_emitter.emit_confidence_update(&update).await?;
            confidence_updates_applied += 1;
        }

        self.storage.store_metrics(&metrics).await?;

        let proposals = self.refiner.propose_refinements(&metrics).await?;
        for proposal in &proposals {
            self.event_emitter.emit_mutation_proposal(proposal).await?;
        }

        self.event_emitter
            .emit_cycle_complete(events.len(), confidence_updates_applied, proposals.len())
            .await?;

        Ok(CycleResult {
            feedback_events_processed: events.len(),
            rules_analyzed: metrics.len(),
            confidence_updates_applied,
            refinement_proposals: proposals.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feedback::FeedbackCollector;

    #[tokio::test]
    async fn test_empty_cycle() {
        let storage = Arc::new(ETLStorage::new());
        let etl = EternalTrainingLoop::new(
            storage,
            Arc::new(RuleConfidenceCalculator),
            Arc::new(RuleConfidenceAdjuster::new()),
            Arc::new(RuleRefiner::new()),
            Arc::new(UniverseEventEmitter::new()),
        );

        let result = etl.run_cycle().await.unwrap();
        assert_eq!(result.feedback_events_processed, 0);
        assert_eq!(result.rules_analyzed, 0);
        assert_eq!(result.confidence_updates_applied, 0);
    }

    #[tokio::test]
    async fn test_cycle_processes_feedback_and_updates_confidence() {
        let storage = Arc::new(ETLStorage::new());
        let collector = FeedbackCollector::new(storage.clone());

        for i in 0..20 {
            collector
                .on_fix_applied(
                    "rule-x".to_string(),
                    "test.rs".to_string(),
                    i,
                    "user-1".to_string(),
                    "success".to_string(),
                )
                .await
                .unwrap();
        }

        let etl = EternalTrainingLoop::new(
            storage.clone(),
            Arc::new(RuleConfidenceCalculator),
            Arc::new(RuleConfidenceAdjuster::new()),
            Arc::new(RuleRefiner::new()),
            Arc::new(UniverseEventEmitter::new()),
        );

        let result = etl.run_cycle().await.unwrap();
        assert_eq!(result.feedback_events_processed, 20);
        assert_eq!(result.rules_analyzed, 1);
        assert_eq!(result.confidence_updates_applied, 1);

        // Metrics should now be cached for the next cycle to compare against.
        let cached = storage.get_metrics("rule-x").await.unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().true_positives, 20);

        // run_cycle() re-scans the whole lookback window each time (no
        // persisted cursor), so a second cycle with no new feedback
        // still reports the same events and re-analyzes the same rule.
        let second = etl.run_cycle().await.unwrap();
        assert_eq!(second.feedback_events_processed, 20);
        assert_eq!(second.rules_analyzed, 1);
    }
}
