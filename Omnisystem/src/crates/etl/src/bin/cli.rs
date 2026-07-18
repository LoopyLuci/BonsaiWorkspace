//! CLI demo for etl: records some feedback events and runs a real
//! EternalTrainingLoop cycle over them.

use etl::{
    ETLStorage, EternalTrainingLoop, FeedbackCollector, RuleConfidenceAdjuster,
    RuleConfidenceCalculator, RuleRefiner, UniverseEventEmitter,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let storage = Arc::new(ETLStorage::new());
    let collector = FeedbackCollector::new(storage.clone());

    for i in 0..15 {
        collector
            .on_fix_applied(
                "no-unused-vars".to_string(),
                "src/main.rs".to_string(),
                i,
                "user-1".to_string(),
                "success".to_string(),
            )
            .await?;
    }
    for i in 0..8 {
        collector
            .on_false_positive_report(
                "no-unused-vars".to_string(),
                "src/main.rs".to_string(),
                100 + i,
                "user-1".to_string(),
                "false positive in generated code".to_string(),
            )
            .await?;
    }

    let etl = EternalTrainingLoop::new(
        storage,
        Arc::new(RuleConfidenceCalculator),
        Arc::new(RuleConfidenceAdjuster::new()),
        Arc::new(RuleRefiner::new()),
        Arc::new(UniverseEventEmitter::new()),
    );

    let result = etl.run_cycle().await?;
    println!(
        "ETL cycle: {} feedback events, {} rules analyzed, {} confidence updates, {} refinement proposals",
        result.feedback_events_processed,
        result.rules_analyzed,
        result.confidence_updates_applied,
        result.refinement_proposals
    );

    Ok(())
}
