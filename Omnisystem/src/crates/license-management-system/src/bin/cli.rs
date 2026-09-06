//! CLI for license-management-system — exercises the crate's real Enterprise processing API.

use license_management_system::Enterprise;

#[tokio::main]
async fn main() -> license_management_system::Result<()> {
    let module = Enterprise::new();
    let input = std::env::args().nth(1).unwrap_or_else(|| "sample payload".to_string());

    let processed = module.process(&input).await?;
    println!("processed: {processed}");

    Ok(())
}
