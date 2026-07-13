//! CLI demo: adapt from HTTP/1 to HTTP/2 based on recorded metrics.

use adaptive_protocols::{
    AdaptationStrategy, CapabilityManager, ProtocolAdapter, ProtocolCapability, ProtocolMetrics,
    ProtocolSelector, ProtocolType,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let selector = ProtocolSelector::new();
    let capabilities = CapabilityManager::new();
    let adapter = ProtocolAdapter::new();

    selector
        .record_metrics(&ProtocolMetrics {
            protocol: ProtocolType::Http1,
            avg_latency_ms: 120.0,
            p99_latency_ms: 250,
            throughput_rps: 400.0,
            success_rate: 0.97,
            error_count: 3,
        })
        .await?;
    selector
        .record_metrics(&ProtocolMetrics {
            protocol: ProtocolType::Http2,
            avg_latency_ms: 45.0,
            p99_latency_ms: 90,
            throughput_rps: 1200.0,
            success_rate: 0.995,
            error_count: 1,
        })
        .await?;

    capabilities
        .register_capability(&ProtocolCapability {
            protocol: ProtocolType::Http2,
            supports_streaming: true,
            supports_multiplexing: true,
            supports_server_push: true,
            max_connections: 100,
            compression_supported: true,
            tls_version: "1.3".to_string(),
        })
        .await?;

    let selected = selector
        .select_protocol(
            "demo-selection",
            ProtocolType::Http1,
            AdaptationStrategy::LatencyOptimized,
        )
        .await?;
    println!("Selected protocol: {:?}", selected);

    let transition = adapter
        .initiate_transition("demo-transition", ProtocolType::Http1, selected, "latency improvement")
        .await?;
    println!("Initiated transition: {:?} -> {:?}", transition.from_protocol, transition.to_protocol);

    adapter.complete_transition("demo-transition", true).await?;
    let completed = adapter.get_transition("demo-transition").await?;
    println!("Transition completed: success={}", completed.success);

    let supports_streaming = capabilities.supports_streaming(selected).await?;
    println!("{:?} supports streaming: {}", selected, supports_streaming);

    Ok(())
}
