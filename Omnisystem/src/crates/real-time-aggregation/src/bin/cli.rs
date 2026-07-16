//! CLI: record a handful of latency samples, then compute a windowed
//! aggregation and percentile breakdown.

use real_time_aggregation::RealTimeAggregator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let aggregator = RealTimeAggregator::new();

    for latency in [12.0, 45.0, 33.0, 88.0, 21.0, 15.0, 200.0] {
        aggregator.record_metric("request_latency_ms", latency).await?;
    }

    let agg = aggregator.aggregate_window("request_latency_ms", 60_000).await?;
    println!(
        "window: count={} sum={:.1} min={:.1} max={:.1} avg={:.2}",
        agg.count, agg.sum, agg.min, agg.max, agg.avg
    );

    let percentiles = aggregator.compute_percentiles("request_latency_ms").await?;
    println!(
        "percentiles: p50={:.1} p95={:.1} p99={:.1} p999={:.1}",
        percentiles.p50, percentiles.p95, percentiles.p99, percentiles.p999
    );

    let rollup = aggregator
        .create_rollup("request_latency_ms", "request_latency_1h", 30)
        .await?;
    println!("rollup created: {} -> {} ({} day retention)", rollup.source_metric, rollup.target_bucket, rollup.retention_days);

    println!("total metrics recorded: {}", aggregator.metric_count());

    Ok(())
}
