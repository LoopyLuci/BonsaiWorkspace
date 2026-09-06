//! CLI for docker-compose-advanced — exercises the crate's real Enterprise processing API.

use docker_compose_advanced::Enterprise;

#[tokio::main]
async fn main() -> docker_compose_advanced::Result<()> {
    let module = Enterprise::new();
    let input = std::env::args().nth(1).unwrap_or_else(|| "sample payload".to_string());

    let processed = module.process(&input).await?;
    println!("processed: {processed}");

    Ok(())
}
