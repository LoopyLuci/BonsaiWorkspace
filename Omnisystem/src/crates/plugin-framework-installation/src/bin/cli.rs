use plugin_framework_installation::Ecosystem;
#[tokio::main] async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let e = Ecosystem::new();
    e.execute().await?;
    Ok(())
}
