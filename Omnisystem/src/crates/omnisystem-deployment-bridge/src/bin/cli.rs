//! CLI for omnisystem-deployment-bridge — exercises the crate's real Enterprise processing API.

use omnisystem_deployment_bridge::Enterprise;

#[tokio::main]
async fn main() -> omnisystem_deployment_bridge::Result<()> {
    let module = Enterprise::new();
    let input = std::env::args().nth(1).unwrap_or_else(|| "sample payload".to_string());

    let processed = module.process(&input).await?;
    println!("processed: {processed}");

    Ok(())
}
