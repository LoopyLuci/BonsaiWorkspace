//! CLI demo: process a sample observability payload through the bridge.

use omnisystem_observability_bridge::Enterprise;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bridge = Enterprise::new();
    let processed = bridge.process("sample-metric-payload").await?;
    println!("Processed: {}", processed);
    Ok(())
}
