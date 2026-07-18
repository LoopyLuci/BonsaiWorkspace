//! Demo CLI: hammers the real per-tenant RPM limiter until it starts rejecting.

use freellmapi_ratelimit::RateLimitService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let limiter = RateLimitService::new();
    let limit = 3;

    for i in 1..=5 {
        let allowed = limiter.check_rpm("demo-tenant", "gpt-4", limit).await?;
        println!("request {i}: allowed={allowed}");
    }

    let (rpm_left, tpm_left) = limiter.get_remaining("demo-tenant", "gpt-4", limit, 10_000).await?;
    println!("remaining rpm={rpm_left} tpm={tpm_left}");

    Ok(())
}
