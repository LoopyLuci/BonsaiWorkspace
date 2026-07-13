//! Universal Language Layer Wrapper for app-manager-core
//!
//! Exposes Rust app-manager-core functions to TITAN and other languages
//! through the Universal Language Layer (ULL) FFI bridge.
//!
//! This follows the same pattern as app-manager-api, showing how to
//! scale the migration approach to all Tier 2 crates.

use universal_language_layer::{
    LanguageBridge, Language,
    ffi::{FunctionSignature, Parameter},
    types::Value,
    error::Result as UllResult,
};
use std::collections::HashMap;

/// Initialize app-manager-core with ULL bridge
///
/// Registers all public Rust functions for TITAN access
pub async fn register_with_ull(bridge: &LanguageBridge) -> UllResult<()> {
    log::info!("Registering app-manager-core with ULL");

    // Register core app management functions
    register_create_app(bridge)?;
    register_update_app(bridge)?;
    register_delete_app(bridge)?;
    register_start_app(bridge)?;
    register_stop_app(bridge)?;

    // Register module itself
    bridge.register_module("app-manager-core", Language::Rust)?;

    log::info!("app-manager-core registered with ULL successfully");
    Ok(())
}

/// Register create_app function
fn register_create_app(bridge: &LanguageBridge) -> UllResult<()> {
    let signature = FunctionSignature {
        name: "create_app".to_string(),
        language: Language::Rust,
        parameters: vec![
            Parameter {
                name: "app_data".to_string(),
                param_type: "object".to_string(),
                required: true,
            },
        ],
        return_type: "object".to_string(),
        is_async: true,
    };

    let _id = bridge.register_function(signature, std::ptr::null())?;
    log::debug!("Registered create_app with ULL");
    Ok(())
}

/// Register update_app function
fn register_update_app(bridge: &LanguageBridge) -> UllResult<()> {
    let signature = FunctionSignature {
        name: "update_app".to_string(),
        language: Language::Rust,
        parameters: vec![
            Parameter {
                name: "app_id".to_string(),
                param_type: "string".to_string(),
                required: true,
            },
            Parameter {
                name: "updates".to_string(),
                param_type: "object".to_string(),
                required: true,
            },
        ],
        return_type: "object".to_string(),
        is_async: true,
    };

    let _id = bridge.register_function(signature, std::ptr::null())?;
    log::debug!("Registered update_app with ULL");
    Ok(())
}

/// Register delete_app function
fn register_delete_app(bridge: &LanguageBridge) -> UllResult<()> {
    let signature = FunctionSignature {
        name: "delete_app".to_string(),
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
    log::debug!("Registered delete_app with ULL");
    Ok(())
}

/// Register start_app function
fn register_start_app(bridge: &LanguageBridge) -> UllResult<()> {
    let signature = FunctionSignature {
        name: "start_app".to_string(),
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
    log::debug!("Registered start_app with ULL");
    Ok(())
}

/// Register stop_app function
fn register_stop_app(bridge: &LanguageBridge) -> UllResult<()> {
    let signature = FunctionSignature {
        name: "stop_app".to_string(),
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
    log::debug!("Registered stop_app with ULL");
    Ok(())
}

/// Convert AppData to ULL Value
pub fn app_data_to_value(
    app_id: &str,
    name: &str,
    version: &str,
) -> Value {
    let mut obj = HashMap::new();

    obj.insert("app_id".to_string(), Value::string(app_id));
    obj.insert("name".to_string(), Value::string(name));
    obj.insert("version".to_string(), Value::string(version));

    Value::object(obj)
}

/// Convert ULL Value to AppData
pub fn value_to_app_data(value: &Value) -> UllResult<(String, String, String)> {
    let obj = value.as_object()?;

    let app_id = obj.get("app_id")
        .and_then(|v| v.as_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let name = obj.get("name")
        .and_then(|v| v.as_str().ok())
        .unwrap_or("Unknown")
        .to_string();

    let version = obj.get("version")
        .and_then(|v| v.as_str().ok())
        .unwrap_or("0.1.0")
        .to_string();

    Ok((app_id, name, version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_data_conversion() {
        let value = app_data_to_value("test-app", "Test App", "1.0.0");
        let (app_id, name, version) = value_to_app_data(&value).unwrap();

        assert_eq!(app_id, "test-app");
        assert_eq!(name, "Test App");
        assert_eq!(version, "1.0.0");
    }
}
