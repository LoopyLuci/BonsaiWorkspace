//! Universal Language Layer Wrapper - Phase 1
use universal_language_layer::{LanguageBridge, Language};

pub async fn register_with_ull(bridge: &LanguageBridge) -> Result<(), String> {
    log::info!("Registering api-gateway-rate-limiting with ULL");
    bridge.register_module("api-gateway-rate-limiting", Language::Rust)
        .map_err(|e| format!("Failed: {}", e))?;
    Ok(())
}
