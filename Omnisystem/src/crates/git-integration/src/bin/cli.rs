//! CLI demo: process a sample git webhook payload through the module.

use git_integration::Enterprise;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let module = Enterprise::new();
    let processed = module.process("sample-push-event").await?;
    println!("Processed: {}", processed);
    Ok(())
}
