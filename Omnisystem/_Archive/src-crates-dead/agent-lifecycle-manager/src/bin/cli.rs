//! CLI for agent-lifecycle-manager

use agent_lifecycle_manager::Component;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let c = Component::new();
    println!("Component initialized successfully");

    let result = c.execute("test").await?;
    println!("Result: {}", result);

    Ok(())
}
