//! CLI

use bmn_gateway::C;

fn main() {
    let c = C::new();
    c.add("key".into(), "value".into());
    println!("Value: {:?}", c.get("key"));
}
