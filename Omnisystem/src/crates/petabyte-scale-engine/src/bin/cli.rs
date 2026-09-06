//! CLI for petabyte-scale-engine — exercises the crate's real in-memory CRUD Manager.

use petabyte_scale_engine::{CreateRequest, Manager, UpdateRequest};

fn main() -> petabyte_scale_engine::Result<()> {
    let manager = Manager::new();

    let created_by = std::env::args().nth(1).unwrap_or_else(|| "cli".to_string());
    let record = manager.create(CreateRequest { created_by: created_by.clone() })?;
    println!("created record {} (by {created_by})", record.id);

    let updated = manager.update(record.id, UpdateRequest { updated_by: "cli-update".to_string() })?;
    println!("updated record {} (by {})", updated.id, updated.updated_by);

    let fetched = manager.get(record.id)?;
    println!("fetched: {:?}", fetched.map(|r| r.id));

    println!("total records: {}", manager.count());
    for r in manager.list() {
        println!("  - {} created_by={}", r.id, r.created_by);
    }

    manager.delete(record.id)?;
    println!("deleted record {}; remaining: {}", record.id, manager.count());

    Ok(())
}
