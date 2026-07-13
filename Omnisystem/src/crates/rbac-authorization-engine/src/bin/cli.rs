//! CLI

use rbac_authorization_engine::Enterprise;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let e = Enterprise::new();
    let result = e.process("test").await?;
    println!("Processed: {}", result);
    Ok(())
}
