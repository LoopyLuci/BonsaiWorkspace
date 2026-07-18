//! Universal Language Layer Wrapper for app-manager-installer
//!
//! Registers app-manager-installer with the Universal Language Layer (ULL)
//! FFI bridge so it can be reached from other Omnisystem languages.

use universal_language_layer::{error::Result as UllResult, Language, LanguageBridge};

/// Register this crate as a module with the ULL bridge.
pub async fn register_with_ull(bridge: &LanguageBridge) -> UllResult<()> {
    log::info!("Registering app-manager-installer with ULL");
    bridge.register_module("app-manager-installer", Language::Rust)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_with_ull() {
        let bridge = LanguageBridge::new();
        let result = register_with_ull(&bridge).await;
        assert!(result.is_ok());
    }
}
