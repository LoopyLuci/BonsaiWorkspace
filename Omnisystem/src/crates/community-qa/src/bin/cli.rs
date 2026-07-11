use community_qa::Component;
#[tokio::main] async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let c = Component::new();
    c.execute("test").await?;
    Ok(())
}
