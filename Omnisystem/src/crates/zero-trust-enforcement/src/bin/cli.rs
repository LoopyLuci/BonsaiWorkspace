//! CLI

use zero_trust_enforcement::Manager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = Manager::new();
    let record = manager.create().await?;
    println!("Created record: {}", record.id);

    let fetched = manager.get(record.id).await?;
    println!("Fetched record: {}", fetched.id);

    println!("Total records: {}", manager.count());
    Ok(())
}
