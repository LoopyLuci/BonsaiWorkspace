//! CLI

use root_cause_analysis_inference::Component;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let c = Component::new();
    println!("Component initialized");

    let result = c.execute("test").await?;
    println!("Result: {}", result);

    Ok(())
}
