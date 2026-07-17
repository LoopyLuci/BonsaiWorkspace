//! CLI demo: set and retrieve a config value through the manager.

use config_management::{ConfigManager, ConfigValue};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = ConfigManager::new();

    manager
        .set_config(&ConfigValue {
            key: "max_connections".to_string(),
            value: "100".to_string(),
        })
        .await?;

    let config = manager.get_config("max_connections").await?;
    println!("{} = {}", config.key, config.value);
    println!("Total configs: {}", manager.config_count());

    Ok(())
}
