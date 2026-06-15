use business_revenue_sharing::Ecosystem;
#[tokio::main] async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let e = Ecosystem::new();
    e.execute().await?;
    Ok(())
}
