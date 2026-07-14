//! CLI demo: initialize the command-executor module.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    command_executor::init().await?;
    println!("command-executor initialized");
    Ok(())
}
