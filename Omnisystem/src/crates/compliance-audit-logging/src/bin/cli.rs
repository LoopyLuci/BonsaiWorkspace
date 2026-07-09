//! CLI

use compliance_audit_logging::Component;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let c = Component::new();
    println!("Component ready");

    c.execute("test").await?;
    println!("Test executed");

    Ok(())
}
