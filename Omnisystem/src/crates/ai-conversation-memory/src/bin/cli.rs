//! CLI demo: analyze and predict on a sample conversation turn.

use ai_conversation_memory::Advanced;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Advanced::new();
    let analysis = engine.analyze("sample-conversation-turn").await?;
    println!("{}", analysis);
    let confidence = engine.predict("sample-conversation-turn").await?;
    println!("Prediction confidence: {}", confidence);
    Ok(())
}
