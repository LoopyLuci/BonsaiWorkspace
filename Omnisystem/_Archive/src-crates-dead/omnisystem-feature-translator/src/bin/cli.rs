//! CLI for omnisystem-feature-translator

use omnisystem_feature_translator::Component;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let c = Component::new();
    println!("Component initialized successfully");

    let result = c.execute("test").await?;
    println!("Result: {}", result);

    Ok(())
}
