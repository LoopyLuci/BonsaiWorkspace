//! Demo CLI: records a few requests into the real DashboardMetrics aggregator and
//! prints the resulting per-tenant/per-provider rollups.

use freellmapi_dashboard::DashboardMetrics;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let metrics = DashboardMetrics::new();

    metrics.record_request("tenant-1", "openai", 0.05, 250).await;
    metrics.record_request("tenant-1", "openai", 0.03, 180).await;
    metrics.record_request("tenant-1", "groq", 0.0, 60).await;

    let tenant = metrics.get_tenant_metrics("tenant-1").await?;
    println!(
        "tenant-1: {} requests, ${:.2} total, {:.1}ms avg latency",
        tenant.total_requests, tenant.total_cost_usd, tenant.avg_latency_ms
    );

    let provider = metrics.get_provider_metrics("openai").await?;
    println!(
        "openai: {} requests, {:.1}ms avg latency",
        provider.total_requests, provider.avg_latency_ms
    );

    Ok(())
}
