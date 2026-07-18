//! Small demo CLI: creates a customer, ingests an event, runs the agent
//! orchestrator, and prints out the resulting decisions and personalization.

use crm_platform::{
    AgentAction, AgentOrchestrator, CrmMetrics, CustomerId, DataSource, IngestionConfig,
    IngestionPipeline, PersonalizationEngine,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let metrics = CrmMetrics::new();

    let pipeline = IngestionPipeline::new(IngestionConfig::default());

    let mut properties = HashMap::new();
    properties.insert("plan".to_string(), "pro".to_string());

    pipeline.ingest_event(crm_platform::cdp::RawEvent {
        source: DataSource::WebAnalytics,
        customer_id: "customer-1".to_string(),
        event_type: "purchase".to_string(),
        timestamp: 1_700_000_000,
        properties,
    })?;
    metrics.record_event();

    let flushed = pipeline.flush_all()?;
    println!("Flushed {flushed} event(s) into the customer store");

    let mut customer = pipeline
        .get_customer("customer-1")
        .expect("customer should exist after flush");
    customer.lifetime_value = 2500.0;
    metrics.record_customer();

    println!(
        "Customer {:?}: health_score={:.2} churn_risk={:.2}",
        customer.primary_id,
        customer.health_score(),
        customer.churn_risk()
    );

    let orchestrator = AgentOrchestrator::new();
    let decisions = orchestrator.execute(&customer);
    for decision in &decisions {
        metrics.record_decision();
        let action = match decision.action {
            AgentAction::Qualify => "Qualify",
            AgentAction::Nurture => "Nurture",
            AgentAction::Reach => "Reach",
            AgentAction::Skip => "Skip",
        };
        println!(
            "  decision: action={action} confidence={:.2} reasoning={}",
            decision.confidence, decision.reasoning
        );
    }

    let recommendations = PersonalizationEngine::get_recommendations(&customer);
    println!("Recommendations: {recommendations:?}");

    let stats = metrics.get_stats();
    println!(
        "Metrics: customers={} events={} decisions={}",
        stats.customers, stats.events, stats.decisions
    );

    // Also verify CustomerId variants are exercised.
    let _ = CustomerId::AnonymousId("anon-1".to_string());

    Ok(())
}
