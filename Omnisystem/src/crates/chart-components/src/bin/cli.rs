//! CLI

use chart_components::{Component, Props};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let c = Component::new(Props::default());
    println!("{}", c.render());
    println!("Props id: {}", c.props().id);
    Ok(())
}
