//! CLI

use omnisystem_committer_core::Core;

fn main() {
    let c = Core::new();
    c.add("key".into(), "value".into());
    println!("Value: {:?}", c.get("key"));
}
