//! CLI

use integration_tests::C;

fn main() {
    let c = C::new();
    c.s("key".into(), "value".into());
    match c.g("key") {
        Some(v) => println!("Fetched: {}", v),
        None => println!("Not found"),
    }
}
