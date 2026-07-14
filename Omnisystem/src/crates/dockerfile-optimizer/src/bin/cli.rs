//! CLI demo: process a sample Dockerfile through the module.

use dockerfile_optimizer::Enterprise;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let module = Enterprise::new();
    let processed = module.process("FROM scratch\nCOPY . /app").await?;
    println!("Processed: {}", processed);
    Ok(())
}
