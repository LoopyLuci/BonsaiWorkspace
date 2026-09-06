//! CLI for continuous-learning-framework — exercises the crate's real Advanced analysis/prediction API.

use continuous_learning_framework::Advanced;

#[tokio::main]
async fn main() -> continuous_learning_framework::Result<()> {
    let engine = Advanced::new();
    let input = std::env::args().nth(1).unwrap_or_else(|| "sample input".to_string());

    let analysis = engine.analyze(&input).await?;
    println!("analysis: {analysis}");

    let score = engine.predict(&input).await?;
    println!("prediction score: {score:.2}");

    Ok(())
}
