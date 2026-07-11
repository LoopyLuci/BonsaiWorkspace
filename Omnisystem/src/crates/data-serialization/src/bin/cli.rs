//! CLI

use data_serialization::{init, Metadata};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init().await?;
    let metadata = Metadata::new();
    println!("Created metadata: {} (v{})", metadata.id, metadata.version);
    Ok(())
}
