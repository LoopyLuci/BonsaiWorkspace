//! CLI for network-policy-manager — exercises the crate's real Enterprise processing API.

use network_policy_manager::Enterprise;

#[tokio::main]
async fn main() -> network_policy_manager::Result<()> {
    let module = Enterprise::new();
    let input = std::env::args().nth(1).unwrap_or_else(|| "sample payload".to_string());

    let processed = module.process(&input).await?;
    println!("processed: {processed}");

    Ok(())
}
