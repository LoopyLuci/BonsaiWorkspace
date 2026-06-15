//! CLI

use particle_swarm_inertia::Component;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let c = Component::new();
    println!("Component initialized");

    let result = c.execute("test").await?;
    println!("Result: {}", result);

    Ok(())
}
