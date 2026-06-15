use bcf::BonsaiContainerFabric;

#[tokio::main]
async fn main() -> bcf::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🚀 Bonsai Container Fabric (BCF) – Next-Generation Container Platform");
    println!("Initializing...");

    // Initialize BCF
    let bcf = BonsaiContainerFabric::new().await?;
    println!("✓ BCF initialized and ready for deployments");

    // Example: Print status
    match bcf.get_service_status("example").await {
        Ok(status) => println!("Service status: {:?}", status),
        Err(_) => println!("No services deployed yet"),
    }

    Ok(())
}
