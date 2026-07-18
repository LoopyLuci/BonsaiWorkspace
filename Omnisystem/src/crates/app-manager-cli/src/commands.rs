//! Command implementations backing the `app-manager-cli` binary.
//!
//! Every command here creates a real `ApplicationManager` and drives real
//! work through it (or, for configuration, a real `app-manager-config`
//! `ConfigManager` backed by real JSON files on disk). Commands with no real
//! backing system in scope (`search`, `logs`, `verify`) return an honest
//! "not yet implemented" error instead of fabricating success output.

use app_manager_config::{AppConfig, ConfigManager};
use app_manager_core::types::{AppId, Version};
use app_manager_omnisystem_integration::ApplicationManager;
use std::path::PathBuf;

/// Base directory persisted per-app JSON configs live under. Mirrors
/// `ApplicationManager`'s own `OMNISYSTEM_APPS_DIR` convention.
fn config_dir() -> PathBuf {
    std::env::var_os("OMNISYSTEM_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./.omnisystem/config"))
}

fn config_file_path(app_id: &AppId) -> PathBuf {
    config_dir().join(format!("{}.json", app_id.as_str()))
}

pub async fn install_command(
    app: &str,
    version: Option<String>,
    _path: Option<PathBuf>,
    _force: bool,
) -> Result<(), String> {
    let app_id = AppId::new(app).map_err(|e| format!("Invalid app ID: {}", e))?;
    let version = version
        .as_ref()
        .and_then(|v| Version::parse(v).ok())
        .unwrap_or_else(|| Version::new(1, 0, 0));

    let mut app_mgr = ApplicationManager::new();
    app_mgr
        .initialize()
        .await
        .map_err(|e| format!("Failed to initialize: {}", e))?;

    app_mgr
        .load_application(&app_id, &version)
        .await
        .map_err(|e| format!("Failed to install: {}", e))?;

    println!("Application {} v{} installed successfully", app, version);
    Ok(())
}

pub async fn uninstall_command(app: &str, _force: bool) -> Result<(), String> {
    let app_id = AppId::new(app).map_err(|e| format!("Invalid app ID: {}", e))?;

    let app_mgr = ApplicationManager::new();
    app_mgr
        .unload_application(&app_id)
        .await
        .map_err(|e| format!("Failed to uninstall: {}", e))?;

    println!("Application {} uninstalled successfully", app);
    Ok(())
}

pub async fn list_command(_filter: Option<String>, _format: &str) -> Result<(), String> {
    let app_mgr = ApplicationManager::new();

    let apps = app_mgr.list_applications();
    let health = app_mgr
        .health_check()
        .await
        .map_err(|e| format!("Failed to check health: {}", e))?;

    println!("Installed Applications");
    println!("  Status:          {}", if health.operational { "Operational" } else { "Degraded" });
    println!("  Modules Loaded:  {}", health.modules_loaded);
    println!("  Initialized:     {}", if health.initialized { "Yes" } else { "No" });

    if apps.is_empty() {
        println!("  (no applications registered in this process)");
    } else {
        for (app_id, state) in apps {
            println!("  {:<24} {:?}", app_id.as_str(), state);
        }
    }

    Ok(())
}

pub async fn status_command(app: &str, detailed: bool) -> Result<(), String> {
    let app_id = AppId::new(app).map_err(|e| format!("Invalid app ID: {}", e))?;

    let app_mgr = ApplicationManager::new();
    let state = app_mgr
        .get_application_state(&app_id)
        .map_err(|e| format!("Failed to get status for {}: {}", app, e))?;

    println!("Application: {}", app);
    println!("  State: {:?}", state);

    if detailed {
        let health = app_mgr
            .health_check()
            .await
            .map_err(|e| format!("Failed to check health: {}", e))?;
        println!("  System modules loaded: {}", health.modules_loaded);
        println!("  System operational:    {}", health.operational);
    }

    Ok(())
}

pub async fn start_command(app: &str) -> Result<(), String> {
    let app_id = AppId::new(app).map_err(|e| format!("Invalid app ID: {}", e))?;

    let app_mgr = ApplicationManager::new();
    app_mgr
        .start_application(&app_id)
        .await
        .map_err(|e| format!("Failed to start: {}", e))?;

    println!("Application {} started successfully", app);
    Ok(())
}

pub async fn stop_command(app: &str) -> Result<(), String> {
    let app_id = AppId::new(app).map_err(|e| format!("Invalid app ID: {}", e))?;

    let app_mgr = ApplicationManager::new();
    app_mgr
        .stop_application(&app_id)
        .await
        .map_err(|e| format!("Failed to stop: {}", e))?;

    println!("Application {} stopped successfully", app);
    Ok(())
}

pub async fn update_command(app: &str, version: Option<String>) -> Result<(), String> {
    let app_id = AppId::new(app).map_err(|e| format!("Invalid app ID: {}", e))?;
    let version = version
        .as_ref()
        .and_then(|v| Version::parse(v).ok())
        .unwrap_or_else(|| Version::new(1, 0, 0));

    let app_mgr = ApplicationManager::new();
    app_mgr
        .update_application(&app_id, &version)
        .await
        .map_err(|e| format!("Failed to update: {}", e))?;

    println!("Application {} updated to v{} successfully", app, version);
    Ok(())
}

/// Rollback IS wired for real, via `ApplicationManager::rollback_application`
/// (which itself delegates to the real `Installer::rollback` /
/// `RollbackManager`).
pub async fn rollback_command(app: &str) -> Result<(), String> {
    let app_id = AppId::new(app).map_err(|e| format!("Invalid app ID: {}", e))?;

    let app_mgr = ApplicationManager::new();
    app_mgr
        .rollback_application(&app_id)
        .await
        .map_err(|e| format!("Failed to rollback: {}", e))?;

    println!("Application {} rolled back successfully", app);
    Ok(())
}

pub async fn health_command() -> Result<(), String> {
    let app_mgr = ApplicationManager::new();
    let health = app_mgr
        .health_check()
        .await
        .map_err(|e| format!("Failed to check health: {}", e))?;

    println!("System Health");
    println!("  Status:          {}", if health.operational { "Operational" } else { "Degraded" });
    println!("  Modules Loaded:  {}", health.modules_loaded);
    println!("  Initialized:     {}", if health.initialized { "Yes" } else { "No" });

    Ok(())
}

/// Real, persisted (per app-id JSON file under `config_dir()`) configuration
/// management backed by `app-manager-config::ConfigManager`. Values set
/// here survive across separate CLI invocations because they are written
/// to disk, not just held in the in-process `ConfigManager`.
pub async fn config_command(
    app: &str,
    set: Option<String>,
    value: Option<String>,
    get: Option<String>,
) -> Result<(), String> {
    let app_id = AppId::new(app).map_err(|e| format!("Invalid app ID: {}", e))?;
    let path = config_file_path(&app_id);

    let manager = ConfigManager::new();

    let mut config = if path.exists() {
        manager
            .load_from_file(&path)
            .await
            .map_err(|e| format!("Failed to load config for {}: {}", app, e))?
    } else {
        AppConfig::new(app_id.as_str().to_string(), "0.0.0".to_string())
    };
    manager
        .save_config(config.clone())
        .map_err(|e| format!("Failed to stage config for {}: {}", app, e))?;

    if let Some(key) = set {
        let val = value.ok_or_else(|| "Value required for set operation".to_string())?;
        config.set_env(key.clone(), val.clone());
        manager
            .update_config(app_id.as_str(), config)
            .map_err(|e| format!("Failed to update config for {}: {}", app, e))?;

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }
        manager
            .save_to_file(app_id.as_str(), &path)
            .await
            .map_err(|e| format!("Failed to persist config for {}: {}", app, e))?;

        println!("Configuration {}={} set for {} (persisted to {:?})", key, val, app, path);
    } else if let Some(key) = get {
        match config.get_env(&key) {
            Some(v) => println!("{}={}", key, v),
            None => println!("{} is not set for {}", key, app),
        }
    } else {
        println!("Configuration for {} ({} entries): {:?}", app, config.environment.len(), config.environment);
    }

    Ok(())
}

/// Not yet implemented: no marketplace search backend is wired into
/// app-manager-cli. Returning a real `Err` instead of fabricating results.
pub async fn search_command(_query: &str, _limit: usize) -> Result<(), String> {
    Err("search is not yet implemented: no marketplace search backend is wired into app-manager-cli".to_string())
}

/// Not yet implemented: no log aggregation/streaming backend exists for
/// installed applications yet.
pub async fn logs_command(_app: &str, _lines: usize, _follow: bool) -> Result<(), String> {
    Err("logs is not yet implemented: no log aggregation backend is wired into app-manager-cli".to_string())
}

/// Not yet implemented: `ApplicationManager` has no standalone
/// post-install integrity-check API (only the checksum check inside the
/// install pipeline itself).
pub async fn verify_command(_app: &str) -> Result<(), String> {
    Err("verify is not yet implemented: ApplicationManager exposes no standalone integrity-check API yet".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_install_command_invalid_app_id() {
        let result = install_command("not a valid id!", None, None, false).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_status_command_unknown_app_errors() {
        let result = status_command("never-installed-app", false).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_health_command_succeeds_on_fresh_manager() {
        let result = health_command().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_search_command_is_honestly_not_implemented() {
        let result = search_command("anything", 10).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not yet implemented"));
    }

    #[tokio::test]
    async fn test_logs_command_is_honestly_not_implemented() {
        let result = logs_command("app", 10, false).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not yet implemented"));
    }

    #[tokio::test]
    async fn test_verify_command_is_honestly_not_implemented() {
        let result = verify_command("app").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not yet implemented"));
    }

    // Both config tests below mutate the process-global OMNISYSTEM_CONFIG_DIR
    // env var that `config_dir()` reads, so they are combined into a single
    // #[tokio::test] to avoid a race against each other under the default
    // parallel test runner (separate #[tokio::test] fns can run
    // concurrently within the same test binary).
    #[tokio::test]
    async fn test_config_command_real_file_persistence() {
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("OMNISYSTEM_CONFIG_DIR", temp.path());

        let app = "cli-config-test-app";

        config_command(app, Some("greeting".to_string()), Some("hello".to_string()), None)
            .await
            .unwrap();

        let app_id = AppId::new(app).unwrap();
        let path = config_file_path(&app_id);
        assert!(path.exists());

        config_command(app, None, None, Some("greeting".to_string()))
            .await
            .unwrap();

        let result = config_command("another-app", Some("key".to_string()), None, None).await;
        assert!(result.is_err());

        std::env::remove_var("OMNISYSTEM_CONFIG_DIR");
    }
}
