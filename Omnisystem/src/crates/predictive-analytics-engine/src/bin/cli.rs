//! CLI for predictive-analytics-engine — exercises the crate's real Service processing/analysis API.

use predictive_analytics_engine::Service;

#[tokio::main]
async fn main() -> predictive_analytics_engine::Result<()> {
    let service = Service::new();
    let input = std::env::args().nth(1).unwrap_or_else(|| "sample input".to_string());

    let processed = service.process(&input).await?;
    println!("processed: {processed}");

    let analysis = service.analyze(&input).await?;
    println!("analysis:  {analysis}");

    Ok(())
}
