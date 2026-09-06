//! CLI for decision-tree-explainer — exercises the crate's real Advanced analysis/prediction API.

use decision_tree_explainer::Advanced;

#[tokio::main]
async fn main() -> decision_tree_explainer::Result<()> {
    let engine = Advanced::new();
    let input = std::env::args().nth(1).unwrap_or_else(|| "sample input".to_string());

    let analysis = engine.analyze(&input).await?;
    println!("analysis: {analysis}");

    let score = engine.predict(&input).await?;
    println!("prediction score: {score:.2}");

    Ok(())
}
