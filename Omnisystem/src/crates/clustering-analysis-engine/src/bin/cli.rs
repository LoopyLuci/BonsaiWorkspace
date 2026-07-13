//! CLI demo: run cluster analysis on sample data.

use clustering_analysis_engine::Advanced;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Advanced::new();
    let analysis = engine.analyze("sample-dataset").await?;
    println!("{}", analysis);
    let confidence = engine.predict("sample-dataset").await?;
    println!("Prediction confidence: {}", confidence);
    Ok(())
}
