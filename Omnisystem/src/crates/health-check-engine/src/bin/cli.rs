//! CLI demo: initialize the health-check-engine module.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    health_check_engine::init().await?;
    println!("health-check-engine initialized");
    Ok(())
}
