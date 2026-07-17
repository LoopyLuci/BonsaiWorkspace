//! CLI demo: process and analyze a sample scheduling request.

use ai_scheduling_optimizer::Service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = Service::new();
    let processed = service.process("sample-job-batch").await?;
    println!("{}", processed);
    let analysis = service.analyze("sample-job-batch").await?;
    println!("{}", analysis);
    Ok(())
}
