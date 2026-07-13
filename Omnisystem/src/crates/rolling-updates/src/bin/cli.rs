//! CLI

use rolling_updates::{CreateRequest, Manager};

fn main() {
    let manager = Manager::new();
    let record = manager
        .create(CreateRequest {
            created_by: "cli".to_string(),
        })
        .expect("create record");
    println!("Created record: {}", record.id);

    println!("Total records: {}", manager.count());
}
