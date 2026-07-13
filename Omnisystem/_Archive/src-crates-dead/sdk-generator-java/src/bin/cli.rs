use sdk_generator_java::Ecosystem;
#[tokio::main] async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let e = Ecosystem::new();
    e.execute().await?;
    Ok(())
}
