//! CLI

use secrets_access_control::Component;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let c = Component::new();
    println!("Component ready");

    c.execute("test").await?;
    println!("Test executed");

    Ok(())
}
