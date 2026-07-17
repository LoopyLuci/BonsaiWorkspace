//! CLI demo: render the accessibility-framework component and handle sample input.

use accessibility_framework::WebComponent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let component = WebComponent::new();
    println!("{}", component.render().await);
    let handled = component.handle("aria-label=submit").await?;
    println!("Handled: {}", handled);
    Ok(())
}
