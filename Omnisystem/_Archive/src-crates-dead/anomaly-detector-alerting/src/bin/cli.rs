//! CLI

use anomaly_detector_alerting::Operations;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ops = Operations::new();
    println!("Operations ready");

    ops.execute("test").await?;
    println!("Test executed");

    Ok(())
}
