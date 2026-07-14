//! CLI demo: render a visualization component and handle sample data.

use visualization_library::WebComponent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let viz = WebComponent::new();
    println!("{}", viz.render().await);
    let handled = viz.handle("{\"series\":[1,2,3]}").await?;
    println!("Handled: {}", handled);
    Ok(())
}
