//! ULL Wrapper for app-manager-security
use universal_language_layer::{LanguageBridge, Language, ffi::FunctionSignature, ffi::Parameter};

pub async fn register_with_ull(bridge: &LanguageBridge) -> Result<(), Box<dyn std::error::Error>> {
    let sig = FunctionSignature {
        name: "check_permission".to_string(),
        language: Language::Rust,
        parameters: vec![
            Parameter {
                name: "user_id".to_string(),
                param_type: "string".to_string(),
                required: true,
            },
            Parameter {
                name: "action".to_string(),
                param_type: "string".to_string(),
                required: true,
            },
        ],
        return_type: "boolean".to_string(),
        is_async: true,
    };
    bridge.register_function(sig, std::ptr::null())?;
    bridge.register_module("app-manager-security", Language::Rust)?;
    Ok(())
}
