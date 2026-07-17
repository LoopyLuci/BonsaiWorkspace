//! CLI demo: execute a hedged request and inspect the outcome.

use chrono::Utc;
use request_hedging::{HedgeManager, HedgeRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = HedgeManager::new();

    let result = manager
        .execute_hedge(&HedgeRequest {
            request_id: "req-1".to_string(),
            service_id: "search-api".to_string(),
            original_deadline: Utc::now(),
            hedge_delay_ms: 5,
            max_hedges: 3,
        })
        .await?;

    println!("Outcome: {:?}, winning attempt: {}", result.outcome, result.winning_attempt);

    Ok(())
}
