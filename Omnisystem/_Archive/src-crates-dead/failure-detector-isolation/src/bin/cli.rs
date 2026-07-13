//! CLI

use failure_detector_isolation::Component;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let c = Component::new();
    println!("Component initialized");

    let result = c.execute("test").await?;
    println!("Result: {}", result);

    Ok(())
}
