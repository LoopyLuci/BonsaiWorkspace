//! CLI

use api_fallback::Core;

fn main() {
    let c = Core::new();
    c.set("key".into(), "value".into());
    println!("Value: {:?}", c.get("key"));
}
