//! CLI demo: acquire a bulkhead permit and inspect its metrics.

use resilience_patterns::{Bulkhead, BulkheadPolicy};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bulkhead = Bulkhead::new("payments-api", BulkheadPolicy::default());

    let _permit = bulkhead.acquire_permit().await?;
    let metrics = bulkhead.get_metrics().await?;
    println!("Active calls: {}, total calls: {}", metrics.active_calls, metrics.total_calls);

    Ok(())
}
