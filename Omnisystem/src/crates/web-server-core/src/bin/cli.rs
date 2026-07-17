//! CLI demo: render the web-server-core component and handle a sample request.

use web_server_core::WebComponent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = WebComponent::new();
    println!("{}", server.render().await);
    let handled = server.handle("GET /health").await?;
    println!("Handled: {}", handled);
    Ok(())
}
