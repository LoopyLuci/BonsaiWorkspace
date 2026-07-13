//! CLI

use srwsts_orchestrator::C;

fn main() {
    let c = C::new();
    c.a("key".into(), "value".into());
    println!("Value: {:?}", c.g("key"));
}
