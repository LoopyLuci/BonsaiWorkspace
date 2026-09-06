//! CLI for container-security-platform — exercises the crate's real Enterprise processing API.

use container_security_platform::Enterprise;

#[tokio::main]
async fn main() -> container_security_platform::Result<()> {
    let module = Enterprise::new();
    let input = std::env::args().nth(1).unwrap_or_else(|| "sample payload".to_string());

    let processed = module.process(&input).await?;
    println!("processed: {processed}");

    Ok(())
}
