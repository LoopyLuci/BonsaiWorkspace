//! CLI demo: process and analyze a sample optimization target.

use automated_optimization_agent::Service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = Service::new();
    let processed = service.process("sample-resource-pool").await?;
    println!("{}", processed);
    let analysis = service.analyze("sample-resource-pool").await?;
    println!("{}", analysis);
    Ok(())
}
