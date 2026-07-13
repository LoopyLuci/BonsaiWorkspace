//! Integration tests for app-manager-api ULL wrapper
//!
//! Tests the bridge between Rust app-manager-api and TITAN wrapper

use app_manager_api::{
    register_with_ull,
    app_info_to_value,
    value_to_app_info,
    models::AppInfo,
};
use universal_language_layer::{LanguageBridge, Language, types::Value};
use std::collections::HashMap;

#[tokio::test]
async fn test_ull_registration() {
    let bridge = LanguageBridge::new();

    // Register app-manager-api module with ULL
    let result = register_with_ull(&bridge).await;
    assert!(result.is_ok(), "Failed to register with ULL");

    // Verify module is registered
    let modules = bridge.list_functions().unwrap();
    assert!(!modules.is_empty(), "No functions registered");
}

#[tokio::test]
async fn test_app_info_conversion() {
    let app = AppInfo {
        app_id: "test-app".to_string(),
        version: "1.0.0".to_string(),
        state: "installed".to_string(),
        installed_at: Some("2026-06-15T12:00:00Z".to_string()),
        running: true,
    };

    // Convert to ULL Value
    let value = app_info_to_value(&app);

    // Verify structure
    assert_eq!(value.value_type, universal_language_layer::types::ValueType::Object);

    // Convert back
    let recovered = value_to_app_info(&value);
    assert!(recovered.is_ok(), "Failed to convert back from ULL Value");

    let app_recovered = recovered.unwrap();
    assert_eq!(app_recovered.app_id, app.app_id);
    assert_eq!(app_recovered.version, app.version);
    assert_eq!(app_recovered.state, app.state);
    assert_eq!(app_recovered.running, app.running);
}

#[tokio::test]
async fn test_app_info_to_value_fields() {
    let app = AppInfo {
        app_id: "my-app".to_string(),
        version: "2.0.0".to_string(),
        state: "running".to_string(),
        installed_at: Some("2026-06-15T10:30:00Z".to_string()),
        running: false,
    };

    let value = app_info_to_value(&app);
    let obj = value.as_object().unwrap();

    // Verify all fields are present
    assert!(obj.contains_key("app_id"));
    assert!(obj.contains_key("version"));
    assert!(obj.contains_key("state"));
    assert!(obj.contains_key("installed_at"));
    assert!(obj.contains_key("running"));

    // Verify field values
    assert_eq!(obj["app_id"].as_str().unwrap(), "my-app");
    assert_eq!(obj["version"].as_str().unwrap(), "2.0.0");
    assert_eq!(obj["state"].as_str().unwrap(), "running");
    assert_eq!(obj["running"].as_bool().unwrap(), false);
}

#[test]
fn test_value_to_app_info_with_missing_fields() {
    // Test with minimal fields
    let mut obj = HashMap::new();
    obj.insert("app_id".to_string(), Value::string("app1"));
    obj.insert("version".to_string(), Value::string("1.0.0"));
    obj.insert("state".to_string(), Value::string("created"));
    obj.insert("running".to_string(), Value::boolean(false));

    let value = Value::object(obj);
    let app = value_to_app_info(&value).unwrap();

    assert_eq!(app.app_id, "app1");
    assert_eq!(app.version, "1.0.0");
    assert_eq!(app.state, "created");
    assert_eq!(app.running, false);
    assert!(app.installed_at.is_none());
}

#[tokio::test]
async fn test_get_app_info_signature() {
    let bridge = LanguageBridge::new();
    register_with_ull(&bridge).await.unwrap();

    // Try to find the get_app_info function
    let signature = bridge.find_function("get_app_info", Language::Rust);
    assert!(signature.is_ok(), "Could not find get_app_info signature");

    let sig = signature.unwrap();
    assert_eq!(sig.name, "get_app_info");
    assert_eq!(sig.language, Language::Rust);
    assert!(sig.is_async);
    assert_eq!(sig.return_type, "object");
}

#[tokio::test]
async fn test_list_apps_signature() {
    let bridge = LanguageBridge::new();
    register_with_ull(&bridge).await.unwrap();

    let signature = bridge.find_function("list_apps", Language::Rust);
    assert!(signature.is_ok());

    let sig = signature.unwrap();
    assert_eq!(sig.name, "list_apps");
    assert!(sig.is_async);
    assert_eq!(sig.return_type, "array");
}

#[tokio::test]
async fn test_install_app_signature() {
    let bridge = LanguageBridge::new();
    register_with_ull(&bridge).await.unwrap();

    let signature = bridge.find_function("install_app", Language::Rust);
    assert!(signature.is_ok());

    let sig = signature.unwrap();
    assert_eq!(sig.name, "install_app");
    assert!(sig.is_async);
    assert!(!sig.parameters.is_empty());
}

#[test]
fn test_app_info_roundtrip() {
    let test_cases = vec![
        AppInfo {
            app_id: "app1".to_string(),
            version: "1.0.0".to_string(),
            state: "installed".to_string(),
            installed_at: Some("2026-06-15T10:00:00Z".to_string()),
            running: true,
        },
        AppInfo {
            app_id: "app2".to_string(),
            version: "2.0.0".to_string(),
            state: "stopped".to_string(),
            installed_at: None,
            running: false,
        },
    ];

    for app in test_cases {
        let value = app_info_to_value(&app);
        let recovered = value_to_app_info(&value).unwrap();

        assert_eq!(app.app_id, recovered.app_id);
        assert_eq!(app.version, recovered.version);
        assert_eq!(app.state, recovered.state);
        assert_eq!(app.running, recovered.running);
        assert_eq!(app.installed_at, recovered.installed_at);
    }
}
