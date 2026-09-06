//! CLI for performance-optimizer — exercises the crate's real WebComponent rendering API.

use performance_optimizer::WebComponent;

#[tokio::main]
async fn main() -> performance_optimizer::Result<()> {
    let component = WebComponent::new();
    println!("rendered: {}", component.render().await);

    let input = std::env::args().nth(1).unwrap_or_else(|| "sample data".to_string());
    let handled = component.handle(&input).await?;
    println!("handled:  {handled}");

    Ok(())
}
