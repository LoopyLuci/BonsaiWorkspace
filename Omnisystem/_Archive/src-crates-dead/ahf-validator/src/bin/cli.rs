//! CLI

use ahf_validator::C;

fn main() {
    let c = C::new();
    c.add("key".into(), "value".into());
    match c.get("key") {
        Some(v) => println!("Fetched: {}", v),
        None => println!("Not found"),
    }
}
