//! CLI demo for ai-advisor: exercises the graceful-degradation Arbiter
//! against a real SovereignService implementation, and the multi-advisor
//! pool routing/aggregation layer.

use ai_advisor::{
    Arbiter, ArbiterConfig, ExecutionResult, ExecutionTier, Result, SovereignService,
};
use ai_advisor::advisor_service::{AdvisorPool, AdvisorRequest, AdvisorService};
use std::collections::HashMap;
use std::sync::Arc;

/// A trivial uppercasing service: deterministic core always succeeds.
struct UppercaseService;

impl SovereignService for UppercaseService {
    fn deterministic_core(&self, input: &[u8]) -> Result<Vec<u8>> {
        Ok(input.to_ascii_uppercase())
    }

    fn name(&self) -> &str {
        "uppercase-service"
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Graceful-degradation ladder: AI disabled by default, so this
    // falls straight through to the deterministic core.
    let mut arbiter = Arbiter::new(ArbiterConfig::default());
    let service = UppercaseService;
    let ExecutionResult { data, tier, confidence } = arbiter.execute(&service, b"hello world");
    println!(
        "Arbiter result: {:?} (tier: {:?}, confidence: {:.2})",
        String::from_utf8_lossy(&data),
        tier,
        confidence
    );
    assert_eq!(tier, ExecutionTier::DeterministicCore);
    println!("Recent decisions logged: {}", arbiter.recent_decisions().len());

    // 2. Multi-advisor pool routing and aggregation.
    let pool = Arc::new(AdvisorPool::new());
    pool.register_advisor("advisor-alpha".to_string(), "general".to_string())
        .await?;
    pool.register_advisor("advisor-beta".to_string(), "general".to_string())
        .await?;

    let service = AdvisorService::new(pool);
    let request = AdvisorRequest {
        request_id: "demo-request".to_string(),
        query: "What's the best way to shard this workload?".to_string(),
        context: HashMap::new(),
        priority: 5,
        timestamp: chrono::Utc::now().timestamp(),
    };

    let aggregated = service.process_request(request).await?;
    println!(
        "Aggregated {} advisor responses, avg confidence {:.2}: {}",
        aggregated.responses.len(),
        aggregated.average_confidence,
        aggregated.consensus
    );

    Ok(())
}
