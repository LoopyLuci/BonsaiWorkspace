//! CLI

use omnisystem_pauser_core::Core;

fn main() {
    let c = Core::new();
    c.add("key".into(), "value".into());
    match c.get("key") {
        Some(v) => println!("Fetched: {}", v),
        None => println!("Not found"),
    }
}
