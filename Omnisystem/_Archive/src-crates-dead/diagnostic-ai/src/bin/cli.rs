//! CLI

use diagnostic_ai::{CreateRequest, Manager};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = Manager::new();
    let record = manager.create(CreateRequest {
        created_by: "cli".to_string(),
    })?;
    println!("Created record: {}", record.id);

    println!("Total records: {}", manager.count());
    Ok(())
}
