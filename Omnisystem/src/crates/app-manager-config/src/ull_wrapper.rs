//! ULL Wrapper for app-manager-config
use universal_language_layer::{LanguageBridge, Language, ffi::FunctionSignature, ffi::Parameter};

pub async fn register_with_ull(bridge: &LanguageBridge) -> Result<(), Box<dyn std::error::Error>> {
    let sig = FunctionSignature {
        name: "load_config".to_string(),
        language: Language::Rust,
        parameters: vec![
            Parameter {
                name: "config_path".to_string(),
                param_type: "string".to_string(),
                required: true,
            },
        ],
        return_type: "object".to_string(),
        is_async: true,
    };
    bridge.register_function(sig, std::ptr::null())?;
    bridge.register_module("app-manager-config", Language::Rust)?;
    Ok(())
}
