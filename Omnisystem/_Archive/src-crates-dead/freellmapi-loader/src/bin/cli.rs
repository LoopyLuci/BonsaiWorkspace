//! CLI

use freellmapi_loader::C;

fn main() {
    let c = C::new();
    c.a("key".into(), "value".into());
    println!("Value: {:?}", c.g("key"));
}
