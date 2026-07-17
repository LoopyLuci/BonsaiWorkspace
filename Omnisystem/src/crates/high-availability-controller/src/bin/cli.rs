//! CLI demo: process a sample failover event through the module.

use high_availability_controller::Enterprise;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let module = Enterprise::new();
    let processed = module.process("sample-failover-event").await?;
    println!("Processed: {}", processed);
    Ok(())
}
