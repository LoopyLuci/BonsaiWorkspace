//! CLI

use anomaly_detection_advanced::Component;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let c = Component::new();
    println!("Component ready");
    c.execute("test").await?;
    println!("Status: {}", c.status());
    Ok(())
}
