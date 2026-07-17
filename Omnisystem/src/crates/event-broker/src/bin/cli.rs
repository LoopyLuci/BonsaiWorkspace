//! CLI demo: create, update, and list records through the in-memory manager.

use event_broker::{CreateRequest, Manager, UpdateRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = Manager::new();

    let record = manager.create(CreateRequest {
        created_by: "demo-user".to_string(),
    })?;
    println!("Created record: {}", record.id);

    manager.update(
        record.id,
        UpdateRequest {
            updated_by: "demo-updater".to_string(),
        },
    )?;
    println!("Updated by: demo-updater");

    let items = manager.list();
    println!("Total records: {}", items.len());

    Ok(())
}
