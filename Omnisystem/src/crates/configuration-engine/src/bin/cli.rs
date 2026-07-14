//! CLI demo: initialize the configuration-engine module.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    configuration_engine::init().await?;
    println!("configuration-engine initialized");
    Ok(())
}
