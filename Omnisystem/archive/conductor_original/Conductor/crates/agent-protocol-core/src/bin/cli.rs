//! CLI for agent-protocol-core

use agent_protocol_core::Module;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let module = Module::new();
    println!("Module initialized successfully");

    let result = module.execute("test").await?;
    println!("Result: {}", result);

    Ok(())
}
