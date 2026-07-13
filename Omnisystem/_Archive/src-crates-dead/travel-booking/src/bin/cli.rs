//! CLI

use travel_booking::Manager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = Manager::new();
    let record = manager.create().await?;
    println!("Created record: {}", record.id);

    let fetched = manager.get(record.id).await?;
    println!("Fetched record: {} (created at {})", fetched.id, fetched.created_at);

    println!("Total records: {}", manager.count());
    Ok(())
}
