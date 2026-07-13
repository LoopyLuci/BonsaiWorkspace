//! CLI

use omnisystem_discovery_service::Core;

fn main() {
    let c = Core::new();
    c.set("key".into(), "value".into());
    println!("Value: {:?}", c.get("key"));
    println!("Count: {}", c.count());
}
