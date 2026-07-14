//! CLI demo: process a sample billing event through the module.

use billing_metering_engine::Enterprise;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let module = Enterprise::new();
    let processed = module.process("sample-usage-event").await?;
    println!("Processed: {}", processed);
    Ok(())
}
