//! CLI

use api_gateway_enterprise::Enterprise;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let enterprise = Enterprise::new();
    let result = enterprise.process("test").await?;
    println!("Processed: {}", result);
    Ok(())
}
