//! CLI for intelligent-command-parser — exercises the crate's real Advanced analysis/prediction API.

use intelligent_command_parser::Advanced;

#[tokio::main]
async fn main() -> intelligent_command_parser::Result<()> {
    let engine = Advanced::new();
    let input = std::env::args().nth(1).unwrap_or_else(|| "sample input".to_string());

    let analysis = engine.analyze(&input).await?;
    println!("analysis: {analysis}");

    let score = engine.predict(&input).await?;
    println!("prediction score: {score:.2}");

    Ok(())
}
