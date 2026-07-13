//! CLI

use bmn_balancer::C;

fn main() {
    let c = C::new();
    c.add("key".into(), "value".into());
    println!("Value: {:?}", c.get("key"));
}
