/// ETL Daemon - Runs EternalTrainingLoop cycles on a schedule
use etl::{
    EternalTrainingLoop, RuleConfidenceCalculator, RuleConfidenceAdjuster, RuleRefiner,
    ETLStorage, UniverseEventEmitter, FeedbackCollector,
};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Bonsai EternalTrainingLoop daemon");

    // Initialize components
    let storage = Arc::new(ETLStorage::new());
    let calculator = Arc::new(RuleConfidenceCalculator);
    let adjuster = Arc::new(RuleConfidenceAdjuster::new());
    let refiner = Arc::new(RuleRefiner::new());
    let event_emitter = Arc::new(UniverseEventEmitter::new());
    let feedback_collector = Arc::new(FeedbackCollector::new(storage.clone()));

    // Create the main ETL orchestrator
    let etl = Arc::new(EternalTrainingLoop::new(
        storage.clone(),
        calculator.clone(),
        adjuster.clone(),
        refiner.clone(),
        event_emitter.clone(),
    ));

    // Run ETL cycles every 24 hours
    let mut scheduler = interval(Duration::from_secs(86400)); // 24 hours

    // Run first cycle immediately for testing/startup verification
    info!("Running initial ETL cycle");
    match etl.run_cycle().await {
        Ok(result) => {
            info!(
                "ETL cycle completed: {} feedback events, {} rules analyzed, {} updates applied",
                result.feedback_events_processed,
                result.rules_analyzed,
                result.confidence_updates_applied
            );
        }
        Err(e) => {
            warn!("ETL cycle failed: {}", e);
        }
    }

    // Then run on schedule
    loop {
        scheduler.tick().await;

        info!("Starting scheduled EternalTrainingLoop cycle");
        match etl.run_cycle().await {
            Ok(result) => {
                info!(
                    "ETL cycle completed: {} feedback events, {} rules analyzed, {} updates applied, {} proposals",
                    result.feedback_events_processed,
                    result.rules_analyzed,
                    result.confidence_updates_applied,
                    result.refinement_proposals
                );

                // Cleanup old events (older than 90 days)
                match storage.cleanup_old_events(90).await {
                    Ok(removed) => info!("Cleaned up {} old feedback events", removed),
                    Err(e) => warn!("Cleanup failed: {}", e),
                }
            }
            Err(e) => {
                warn!("ETL cycle failed: {}", e);
            }
        }
    }
}
