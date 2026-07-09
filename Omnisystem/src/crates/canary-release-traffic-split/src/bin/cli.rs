//! CLI

use canary_release_traffic_split::Component;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let c = Component::new();
    println!("Component ready");
    c.execute("test").await?;
    println!("Status: {}", c.status());
    Ok(())
}
