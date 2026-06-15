//! CLI

use cost_optimization_agent::Component;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let c = Component::new();
    println!("Component ready");
    c.execute("test").await?;
    println!("Status: {}", c.status());
    Ok(())
}
