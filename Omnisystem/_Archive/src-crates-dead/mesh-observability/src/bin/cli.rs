//! CLI

use mesh_observability::DistributedTracer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tracer = DistributedTracer::new();
    let trace_id = tracer.start_trace("api", "GET /health").await?;
    println!("Started trace: {}", trace_id);

    tracer.end_trace(trace_id, 42).await?;
    println!("Total traces: {}", tracer.trace_count());

    Ok(())
}
