//! CLI demo: process a sample tenant provisioning request through the module.

use multi_tenancy_engine::Enterprise;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let module = Enterprise::new();
    let processed = module.process("sample-tenant-provisioning-request").await?;
    println!("Processed: {}", processed);
    Ok(())
}
