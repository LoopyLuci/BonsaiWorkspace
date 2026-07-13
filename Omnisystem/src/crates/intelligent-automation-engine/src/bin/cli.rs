//! CLI demo: run automated analysis and prediction on sample data.

use intelligent_automation_engine::Advanced;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Advanced::new();
    let analysis = engine.analyze("sample-workflow").await?;
    println!("{}", analysis);
    let confidence = engine.predict("sample-workflow").await?;
    println!("Prediction confidence: {}", confidence);
    Ok(())
}
