//! ULL Wrapper for app-manager-cli
use universal_language_layer::{LanguageBridge, Language, ffi::FunctionSignature};

pub async fn register_with_ull(bridge: &LanguageBridge) -> Result<(), Box<dyn std::error::Error>> {
    let sig = FunctionSignature {
        name: "execute_cli".to_string(),
        language: Language::Rust,
        parameters: vec![],
        return_type: "string".to_string(),
        is_async: true,
    };
    bridge.register_function(sig, std::ptr::null())?;
    bridge.register_module("app-manager-cli", Language::Rust)?;
    Ok(())
}
