//! CLI for security-analyzer — exercises the crate's real Service processing/analysis API.

use security_analyzer::Service;

#[tokio::main]
async fn main() -> security_analyzer::Result<()> {
    let service = Service::new();
    let input = std::env::args().nth(1).unwrap_or_else(|| "sample input".to_string());

    let processed = service.process(&input).await?;
    println!("processed: {processed}");

    let analysis = service.analyze(&input).await?;
    println!("analysis:  {analysis}");

    Ok(())
}
