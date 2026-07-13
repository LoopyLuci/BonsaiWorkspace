//! CLI

use event_processing_engine::init;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init().await?;
    println!("event-processing-engine initialized");
    Ok(())
}
