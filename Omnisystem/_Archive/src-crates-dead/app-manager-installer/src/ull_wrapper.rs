//! Universal Language Layer Wrapper
//! Auto-generated Phase 1 bridge layer

use universal_language_layer::{LanguageBridge, Language};

pub async fn register_with_ull(bridge: &LanguageBridge) -> Result<(), String> {
    log::info!("Registering module with ULL");
    bridge.register_module("app-manager-installer", Language::Rust)
        .map_err(|e| format!("Registration failed: {}", e))?;
    Ok(())
}
