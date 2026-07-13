//! CLI

use rate_limiting::{RequestPriority, TokenBucketConfig, TokenBucketLimiter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let limiter = TokenBucketLimiter::new();
    let config = TokenBucketConfig {
        capacity: 100,
        refill_rate: 10,
        refill_interval_ms: 1000,
    };

    limiter.create_bucket("bucket-1", &config).await?;
    let decision = limiter
        .allow_request("bucket-1", 10, RequestPriority::Normal)
        .await?;
    println!("Allowed: {}, tokens remaining: {}", decision.allowed, decision.tokens_remaining);

    println!("Total buckets: {}", limiter.bucket_count());
    Ok(())
}
