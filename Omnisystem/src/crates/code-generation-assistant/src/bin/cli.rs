//! CLI

use code_generation_assistant::Advanced;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a = Advanced::new();
    println!("Advanced ready");
    let analyzed = a.analyze("test").await?;
    println!("{}", analyzed);
    let predicted = a.predict("test").await?;
    println!("Prediction: {}", predicted);
    Ok(())
}
