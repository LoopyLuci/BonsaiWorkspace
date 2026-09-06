//! CLI for kubernetes-integration-layer — exercises the crate's real Enterprise processing API.

use kubernetes_integration_layer::Enterprise;

#[tokio::main]
async fn main() -> kubernetes_integration_layer::Result<()> {
    let module = Enterprise::new();
    let input = std::env::args().nth(1).unwrap_or_else(|| "sample payload".to_string());

    let processed = module.process(&input).await?;
    println!("processed: {processed}");

    Ok(())
}
