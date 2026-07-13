//! Universal Language Layer Wrapper for app-manager-api
//!
//! Exposes Rust app-manager-api functions to TITAN and other languages
//! through the Universal Language Layer (ULL) FFI bridge.

use universal_language_layer::{
    LanguageBridge, Language,
    ffi::{FunctionSignature, Parameter},
    types::Value,
    error::Result as UllResult,
};
use crate::models::AppInfo;
use std::collections::HashMap;

/// Initialize app-manager-api with ULL bridge
///
/// Registers all public Rust functions for TITAN access
pub async fn register_with_ull(bridge: &LanguageBridge) -> UllResult<()> {
    log::info!("Registering app-manager-api with ULL");

    // Register: get_app_info(app_id: String) -> AppInfo
    register_get_app_info(bridge)?;

    // Register: list_apps() -> Array[AppInfo]
    register_list_apps(bridge)?;

    // Register: install_app(app: AppInfo) -> InstallationInfo
    register_install_app(bridge)?;

    // Register: uninstall_app(app_id: String) -> bool
    register_uninstall_app(bridge)?;

    // Register module itself
    bridge.register_module("app-manager-api", Language::Rust)?;

    log::info!("app-manager-api registered with ULL successfully");
    Ok(())
}

/// Register get_app_info function
fn register_get_app_info(bridge: &LanguageBridge) -> UllResult<()> {
    let signature = FunctionSignature {
        name: "get_app_info".to_string(),
        language: Language::Rust,
        parameters: vec![
            Parameter {
                name: "app_id".to_string(),
                param_type: "string".to_string(),
                required: true,
            },
        ],
        return_type: "object".to_string(),
        is_async: true,
    };

    // In production, would register actual function pointer
    // For now, register metadata only
    let _id = bridge.register_function(signature, std::ptr::null())?;

    log::debug!("Registered get_app_info with ULL");
    Ok(())
}

/// Register list_apps function
fn register_list_apps(bridge: &LanguageBridge) -> UllResult<()> {
    let signature = FunctionSignature {
        name: "list_apps".to_string(),
        language: Language::Rust,
        parameters: vec![],
        return_type: "array".to_string(),
        is_async: true,
    };

    let _id = bridge.register_function(signature, std::ptr::null())?;
    log::debug!("Registered list_apps with ULL");
    Ok(())
}

/// Register install_app function
fn register_install_app(bridge: &LanguageBridge) -> UllResult<()> {
    let signature = FunctionSignature {
        name: "install_app".to_string(),
        language: Language::Rust,
        parameters: vec![
            Parameter {
                name: "app".to_string(),
                param_type: "object".to_string(),
                required: true,
            },
        ],
        return_type: "object".to_string(),
        is_async: true,
    };

    let _id = bridge.register_function(signature, std::ptr::null())?;
    log::debug!("Registered install_app with ULL");
    Ok(())
}

/// Register uninstall_app function
fn register_uninstall_app(bridge: &LanguageBridge) -> UllResult<()> {
    let signature = FunctionSignature {
        name: "uninstall_app".to_string(),
        language: Language::Rust,
        parameters: vec![
            Parameter {
                name: "app_id".to_string(),
                param_type: "string".to_string(),
                required: true,
            },
        ],
        return_type: "boolean".to_string(),
        is_async: true,
    };

    let _id = bridge.register_function(signature, std::ptr::null())?;
    log::debug!("Registered uninstall_app with ULL");
    Ok(())
}

/// Convert AppInfo to ULL Value
pub fn app_info_to_value(app: &AppInfo) -> Value {
    let mut obj = HashMap::new();

    obj.insert("app_id".to_string(), Value::string(&app.app_id));
    obj.insert("version".to_string(), Value::string(&app.version));
    obj.insert("state".to_string(), Value::string(&app.state));
    obj.insert("running".to_string(), Value::boolean(app.running));

    if let Some(installed_at) = &app.installed_at {
        obj.insert("installed_at".to_string(), Value::string(installed_at));
    }

    Value::object(obj)
}

/// Convert ULL Value to AppInfo
pub fn value_to_app_info(value: &Value) -> UllResult<AppInfo> {
    let obj = value.as_object()?;

    Ok(AppInfo {
        app_id: obj.get("app_id")
            .and_then(|v| v.as_str().ok())
            .unwrap_or("unknown")
            .to_string(),
        version: obj.get("version")
            .and_then(|v| v.as_str().ok())
            .unwrap_or("0.1.0")
            .to_string(),
        state: obj.get("state")
            .and_then(|v| v.as_str().ok())
            .unwrap_or("unknown")
            .to_string(),
        installed_at: obj.get("installed_at")
            .and_then(|v| v.as_str().ok())
            .map(|s| s.to_string()),
        running: obj.get("running")
            .and_then(|v| v.as_bool().ok())
            .unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_info_conversion() {
        let app = AppInfo {
            app_id: "test-app".to_string(),
            version: "1.0.0".to_string(),
            state: "installed".to_string(),
            installed_at: Some("2026-06-15T12:00:00Z".to_string()),
            running: true,
        };

        let value = app_info_to_value(&app);
        let recovered = value_to_app_info(&value).unwrap();

        assert_eq!(recovered.app_id, app.app_id);
        assert_eq!(recovered.version, app.version);
        assert_eq!(recovered.running, app.running);
    }
}
