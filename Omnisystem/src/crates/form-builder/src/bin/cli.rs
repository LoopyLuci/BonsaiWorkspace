//! CLI demo: render a form and handle a submission.

use form_builder::WebComponent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let form = WebComponent::new();
    println!("{}", form.render().await);
    let handled = form.handle("field=value").await?;
    println!("Handled: {}", handled);
    Ok(())
}
