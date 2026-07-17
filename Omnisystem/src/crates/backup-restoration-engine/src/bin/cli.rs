//! CLI demo: initialize the backup-restoration-engine module.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    backup_restoration_engine::init().await?;
    println!("backup-restoration-engine initialized");
    Ok(())
}
