//! CLI demo: run a forecast analysis on sample data.

use forecasting_engine_advanced::Advanced;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Advanced::new();
    let analysis = engine.analyze("sample-demand-series").await?;
    println!("{}", analysis);
    let confidence = engine.predict("sample-demand-series").await?;
    println!("Prediction confidence: {}", confidence);
    Ok(())
}
