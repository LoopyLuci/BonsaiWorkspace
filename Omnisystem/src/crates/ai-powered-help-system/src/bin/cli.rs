//! CLI demo: run a help-content analysis on a sample query.

use ai_powered_help_system::Advanced;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Advanced::new();
    let analysis = engine.analyze("how do I reset my password").await?;
    println!("{}", analysis);
    let confidence = engine.predict("how do I reset my password").await?;
    println!("Prediction confidence: {}", confidence);
    Ok(())
}
