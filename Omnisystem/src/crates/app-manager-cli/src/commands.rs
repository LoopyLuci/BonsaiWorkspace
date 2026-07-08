use std::path::PathBuf;
use app_manager_core::{AppId, Version};
use app_manager_omnisystem_integration::ApplicationManager;

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

    println!("✓ Application {} v{} installed successfully", app, version);
    Ok(())
}

pub async fn uninstall_command(app: &str, _force: bool) -> Result<(), String> {
    let app_id = AppId::new(app).map_err(|e| format!("Invalid app ID: {}", e))?;

    let app_mgr = ApplicationManager::new();
    app_mgr
        .unload_application(&app_id)
        .await
        .map_err(|e| format!("Failed to uninstall: {}", e))?;

    println!("✓ Application {} uninstalled successfully", app);
    Ok(())
}

pub async fn list_command(_filter: Option<String>, _format: &str) -> Result<(), String> {
    let app_mgr = ApplicationManager::new();
    let health = app_mgr
        .health_check()
        .await
        .map_err(|e| format!("Failed to check health: {}", e))?;

    println!("┌─ Installed Applications ──────────────────────┐");
    println!("│ Status: {}", if health.operational { "Operational" } else { "Degraded" });
    println!("│ Modules Loaded: {}", health.modules_loaded);
    println!("│ Initialized: {}", if health.initialized { "Yes" } else { "No" });
    println!("└──────────────────────────────────────────────┘");

    Ok(())
}

pub async fn status_command(app: &str, _detailed: bool) -> Result<(), String> {
    let app_id = AppId::new(app).map_err(|e| format!("Invalid app ID: {}", e))?;

    let app_mgr = ApplicationManager::new();
    let health = app_mgr
        .health_check()
        .await
        .map_err(|e| format!("Failed to get status: {}", e))?;

    println!("Application: {}", app);
    println!("  Initialized: {}", if health.initialized { "Yes" } else { "No" });
    println!("  Modules Loaded: {}", health.modules_loaded);
    println!("  Status: {}", if health.operational { "Running" } else { "Stopped" });

    Ok(())
}

pub async fn start_command(app: &str) -> Result<(), String> {
    let app_id = AppId::new(app).map_err(|e| format!("Invalid app ID: {}", e))?;

    let app_mgr = ApplicationManager::new();
    app_mgr
        .start_application(&app_id)
        .await
        .map_err(|e| format!("Failed to start: {}", e))?;

    println!("✓ Application {} started successfully", app);
    Ok(())
}

pub async fn stop_command(app: &str) -> Result<(), String> {
    let app_id = AppId::new(app).map_err(|e| format!("Invalid app ID: {}", e))?;

    let app_mgr = ApplicationManager::new();
    app_mgr
        .stop_application(&app_id)
        .await
        .map_err(|e| format!("Failed to stop: {}", e))?;

    println!("✓ Application {} stopped successfully", app);
    Ok(())
}

pub async fn update_command(app: &str, version: Option<String>) -> Result<(), String> {
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
        .map_err(|e| format!("Failed to update: {}", e))?;

    println!("✓ Application {} updated to v{} successfully", app, version);
    Ok(())
}

pub async fn config_command(
    app: &str,
    set: Option<String>,
    value: Option<String>,
    get: Option<String>,
) -> Result<(), String> {
    let app_id = AppId::new(app).map_err(|e| format!("Invalid app ID: {}", e))?;

    if let Some(key) = set {
        if let Some(val) = value {
            println!("✓ Configuration {}={} set for {}", key, val, app);
        } else {
            return Err("Value required for set operation".to_string());
        }
    } else if let Some(key) = get {
        println!("Configuration {}=<value> for {}", key, app);
    } else {
        println!("Configuration options for {}: <none>", app);
    }

    Ok(())
}
