//! CLI for natural-language-interface

use natural_language_interface::Module;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let module = Module::new();
    println!("Module initialized successfully");

    let result = module.execute("test").await?;
    println!("Result: {}", result);

    Ok(())
}
