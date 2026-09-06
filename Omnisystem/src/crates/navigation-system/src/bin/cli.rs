//! CLI for navigation-system — exercises the crate's real WebComponent rendering API.

use navigation_system::WebComponent;

#[tokio::main]
async fn main() -> navigation_system::Result<()> {
    let component = WebComponent::new();
    println!("rendered: {}", component.render().await);

    let input = std::env::args().nth(1).unwrap_or_else(|| "sample data".to_string());
    let handled = component.handle(&input).await?;
    println!("handled:  {handled}");

    Ok(())
}
