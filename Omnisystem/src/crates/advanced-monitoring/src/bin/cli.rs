//! CLI demo: record a metric and run analytics over it.

use advanced_monitoring::{AnalyticsEngine, Metric, MetricsCollector};
use chrono::Utc;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let collector = MetricsCollector::new();
    let analytics = AnalyticsEngine::new();

    collector
        .record(&Metric {
            metric_name: "cpu_usage".to_string(),
            value: 72.5,
            timestamp: Utc::now(),
            labels: HashMap::new(),
        })
        .await?;

    let metrics = collector.get_metrics("cpu_usage").await?;
    println!("Recorded {} metric(s)", metrics.len());

    let result = analytics.analyze(&metrics).await?;
    println!("Analysis result: {:?}", result);

    Ok(())
}
