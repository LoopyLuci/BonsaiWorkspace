//! CLI for intelligent-recommendation-system — exercises the crate's real Service processing/analysis API.

use intelligent_recommendation_system::Service;

#[tokio::main]
async fn main() -> intelligent_recommendation_system::Result<()> {
    let service = Service::new();
    let input = std::env::args().nth(1).unwrap_or_else(|| "sample input".to_string());

    let processed = service.process(&input).await?;
    println!("processed: {processed}");

    let analysis = service.analyze(&input).await?;
    println!("analysis:  {analysis}");

    Ok(())
}
