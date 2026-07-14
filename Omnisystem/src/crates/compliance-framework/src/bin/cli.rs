//! CLI demo: process a sample compliance record through the module.

use compliance_framework::Enterprise;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let module = Enterprise::new();
    let processed = module.process("sample-compliance-record").await?;
    println!("Processed: {}", processed);
    Ok(())
}
