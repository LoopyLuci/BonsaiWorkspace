//! CLI

use backup_orchestrator::Manager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = Manager::new();
    let record = manager.create().await?;
    println!("Created record: {}", record.id);

    println!("Total records: {}", manager.count());
    Ok(())
}
