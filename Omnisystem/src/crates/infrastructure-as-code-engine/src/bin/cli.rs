//! CLI for infrastructure-as-code-engine — exercises the crate's real Enterprise processing API.

use infrastructure_as_code_engine::Enterprise;

#[tokio::main]
async fn main() -> infrastructure_as_code_engine::Result<()> {
    let module = Enterprise::new();
    let input = std::env::args().nth(1).unwrap_or_else(|| "sample payload".to_string());

    let processed = module.process(&input).await?;
    println!("processed: {processed}");

    Ok(())
}
