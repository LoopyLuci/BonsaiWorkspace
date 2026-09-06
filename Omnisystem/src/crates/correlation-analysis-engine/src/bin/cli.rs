//! CLI for correlation-analysis-engine — exercises the crate's real Advanced analysis/prediction API.

use correlation_analysis_engine::Advanced;

#[tokio::main]
async fn main() -> correlation_analysis_engine::Result<()> {
    let engine = Advanced::new();
    let input = std::env::args().nth(1).unwrap_or_else(|| "sample input".to_string());

    let analysis = engine.analyze(&input).await?;
    println!("analysis: {analysis}");

    let score = engine.predict(&input).await?;
    println!("prediction score: {score:.2}");

    Ok(())
}
