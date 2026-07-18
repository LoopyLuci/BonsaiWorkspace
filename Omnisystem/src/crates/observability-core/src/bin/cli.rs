//! Observability Core CLI - exercises tracing, logging, metrics, and correlation

use observability_core::{
    CorrelationManager, DistributedTracer, LogCollector, LogLevel, MetricsAggregator, SpanId,
    TraceId,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tracer = DistributedTracer::new();
    let trace_id = TraceId(uuid::Uuid::new_v4().to_string());
    let span_id = SpanId(uuid::Uuid::new_v4().to_string());
    tracer.start_trace(&trace_id, &span_id, "cli-demo").await?;
    tracer.end_span(&span_id).await?;
    println!("trace spans: {}", tracer.span_count());

    let logs = LogCollector::new(100);
    logs.write_log(&observability_core::LogEntry {
        timestamp: chrono::Utc::now(),
        level: LogLevel::Info,
        message: "observability core CLI demo".to_string(),
        service: "observability-core-cli".to_string(),
        trace_id: Some(trace_id.clone()),
        span_id: Some(span_id.clone()),
        correlation_id: None,
        fields: HashMap::new(),
    })
    .await?;
    println!("logs recorded: {}", logs.log_count());

    let metrics = MetricsAggregator::new();
    metrics.record_metric("demo_latency_ms", 42.0, HashMap::new()).await?;
    let agg = metrics.aggregate_metrics("demo_latency_ms").await?;
    println!("metric p50: {:.1}", agg.p50);

    let correlation = CorrelationManager::new();
    let correlation_id = correlation.create_context(&trace_id, &span_id).await?;
    correlation.set_baggage(&correlation_id, "user", "demo").await?;
    println!("baggage user: {:?}", correlation.get_baggage(&correlation_id, "user").await?);

    Ok(())
}
