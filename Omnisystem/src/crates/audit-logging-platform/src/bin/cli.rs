//! CLI demo: process a sample audit log entry through the module.

use audit_logging_platform::Enterprise;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let module = Enterprise::new();
    let processed = module.process("sample-audit-entry").await?;
    println!("Processed: {}", processed);
    Ok(())
}
