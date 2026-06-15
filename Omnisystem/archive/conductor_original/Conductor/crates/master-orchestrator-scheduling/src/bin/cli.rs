//! CLI

use master_orchestrator_scheduling::Component;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let c = Component::new();
    println!("Autonomous component ready");

    c.execute().await?;
    println!("Status: {}", c.status());

    Ok(())
}
