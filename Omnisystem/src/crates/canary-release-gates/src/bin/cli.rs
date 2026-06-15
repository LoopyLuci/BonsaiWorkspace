//! CLI

use canary_release_gates::Operations;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ops = Operations::new();
    println!("Operations ready");

    ops.execute("test").await?;
    println!("Test executed");

    Ok(())
}
