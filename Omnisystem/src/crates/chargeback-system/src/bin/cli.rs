//! CLI

use chargeback_system::ChargebackManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = ChargebackManager::new();
    let allocation_id = manager.allocate_cost("dept1", "team1", 1000.0, 50.0).await?;
    println!("Created allocation: {}", allocation_id);
    println!("Total allocations: {}", manager.allocation_count());

    Ok(())
}
