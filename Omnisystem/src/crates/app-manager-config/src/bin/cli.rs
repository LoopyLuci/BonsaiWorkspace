//! App Manager Config CLI: creates a config, saves it, tweaks resources
//! and environment, round-trips it through TOML on disk, and prints it.

use app_manager_config::{AppConfig, ConfigManager, EnvironmentManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = AppConfig::new("bonsai-workspace".to_string(), "2.0.0".to_string());
    config.set_resources(4, 2048, 4096);
    config.set_feature("telemetry".to_string(), false);
    config.set_env("RUST_LOG".to_string(), "info".to_string());

    let manager = ConfigManager::new();
    manager.save_config(config.clone())?;
    println!("saved config for {} v{}", config.app_id, config.version);

    let tmp = std::env::temp_dir().join("app-manager-config-cli-demo.toml");
    manager.save_to_file(&config.app_id, &tmp).await?;
    println!("wrote config to {}", tmp.display());

    let reloaded = manager.load_from_file(&tmp).await?;
    println!(
        "reloaded: cpu_cores={} memory_mb={} telemetry={}",
        reloaded.resources.cpu_cores,
        reloaded.resources.memory_mb,
        reloaded.is_feature_enabled("telemetry")
    );

    let env_mgr = EnvironmentManager::new();
    env_mgr.set_variable(&config.app_id, "RUST_LOG".to_string(), "debug".to_string())?;
    env_mgr.set_variable(&config.app_id, "PORT".to_string(), "8080".to_string())?;
    println!("\nshell export:\n{}", env_mgr.export_to_shell(&config.app_id)?);

    let _ = std::fs::remove_file(&tmp);
    Ok(())
}
