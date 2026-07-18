//! CLI demo: exercises the ObservabilityStack end-to-end.

use observability::{ObservabilityStack, SLATarget};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target = SLATarget {
        p95_latency_ms: 100.0,
        p99_latency_ms: 200.0,
        availability_percent: 99.9,
    };

    let stack = ObservabilityStack::new(target);
    stack.initialize().await.map_err(|e| e.to_string())?;

    stack.record_operation("api_request", 45.0, true);
    stack.record_operation("api_request", 52.0, true);
    stack.record_operation("api_request", 210.0, false);

    let compliance = stack.get_sla_compliance();
    println!(
        "SLA compliance: {:.1}% (p95={:.1}ms p99={:.1}ms availability={:.2}%)",
        compliance.compliance_percent,
        compliance.current_p95_ms,
        compliance.current_p99_ms,
        compliance.current_availability_percent
    );

    let prometheus = stack.export_prometheus().await.map_err(|e| e.to_string())?;
    println!("--- Prometheus export ---\n{}", prometheus);

    Ok(())
}
