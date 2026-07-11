//! CLI

use pathfinder_auth::C;

fn main() {
    let c = C::new();
    c.a("key".into(), "value".into());
    match c.g("key") {
        Some(v) => println!("Fetched: {}", v),
        None => println!("Not found"),
    }
}
