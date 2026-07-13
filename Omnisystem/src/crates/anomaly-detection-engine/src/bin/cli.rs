//! CLI

use anomaly_detection_engine::Service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let s = Service::new();
    let processed = s.process("test").await?;
    println!("{}", processed);
    let analyzed = s.analyze("data").await?;
    println!("{}", analyzed);
    Ok(())
}
