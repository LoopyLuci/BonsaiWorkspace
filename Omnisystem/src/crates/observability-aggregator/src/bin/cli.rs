//! CLI demo: record metrics and aggregate them into a window.

use observability_aggregator::{AggregatorConfig, MetricsAggregator};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let aggregator = MetricsAggregator::new(AggregatorConfig::default());

    aggregator
        .record_metric("request_latency_ms", 42.0, HashMap::new())
        .await?;
    aggregator
        .record_metric("request_latency_ms", 58.0, HashMap::new())
        .await?;

    let series = aggregator.get_time_series("request_latency_ms").await?;
    println!("Recorded {} metric(s), {} window(s)", aggregator.metric_count(), series.windows.len());

    let stats = aggregator.get_stats().await?;
    println!("Total data points: {}", stats.total_data_points);

    Ok(())
}
