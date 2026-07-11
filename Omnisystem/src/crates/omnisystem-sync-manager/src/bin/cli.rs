//! CLI

use omnisystem_sync_manager::Core;

fn main() {
    let c = Core::new();
    c.set("key".into(), "value".into());
    match c.get("key") {
        Some(v) => println!("Fetched: {}", v),
        None => println!("Not found"),
    }
    println!("Total: {}", c.count());
}
