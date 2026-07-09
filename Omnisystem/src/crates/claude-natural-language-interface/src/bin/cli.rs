//! CLI

use claude_natural_language_interface::Advanced;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let advanced = Advanced::new();
    let result = advanced.analyze("test").await?;
    println!("Analyzed: {}", result);
    let prediction = advanced.predict("test").await?;
    println!("Prediction: {}", prediction);
    Ok(())
}
