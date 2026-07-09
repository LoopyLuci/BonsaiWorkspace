//! CLI

use omnisystem_markup::Core;

fn main() {
    let c = Core::new();
    c.set("key".into(), "value".into());
    match c.get("key") {
        Some(v) => println!("Fetched: {}", v),
        None => println!("Not found"),
    }
}
