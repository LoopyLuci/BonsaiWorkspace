//! Universal Language Layer (ULL)
//!
//! A unified FFI and interoperability layer enabling seamless communication
//! between Rust, TITAN, SYLVA, AETHER, and AXIOM languages.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │         Universal Language Layer (ULL)                   │
//! │  ┌───────────────────────────────────────────────────┐  │
//! │  │  FFI Bridge | Type System | Error Handling | Async  │  │
//! │  └───────────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────┘
//!        ↙         ↓          ↓         ↓          ↘
//!      Rust     TITAN      SYLVA    AETHER      AXIOM
//! ```
//!
//! # Features
//!
//! - **Language Agnostic** — Call functions across any language
//! - **Type Safe** — Compile-time and runtime type checking
//! - **Memory Safe** — Automatic memory management across language boundaries
//! - **Async Ready** — Native async/await support
//! - **Error Handling** — Unified error system
//! - **Performance** — Zero-copy where possible
//! - **Hot Reload** — Dynamic library loading/unloading

pub mod bridge;
pub mod error;
pub mod ffi;
pub mod language;
pub mod registry;
pub mod types;

pub use bridge::{LanguageBridge, BridgeBuilder};
pub use error::{UllError, Result};
pub use language::{Language, LanguageContext};
pub use registry::LanguageRegistry;
pub use types::{Value, ValueType};

/// ULL Version
pub const VERSION: &str = "1.0.0";

/// Initialize the Universal Language Layer
pub async fn initialize() -> Result<()> {
    log::info!("Initializing Universal Language Layer v{}", VERSION);

    // Initialize language runtimes
    language::initialize_runtimes().await?;

    log::info!("Universal Language Layer initialized successfully");
    Ok(())
}

/// Shutdown the Universal Language Layer
pub async fn shutdown() -> Result<()> {
    log::info!("Shutting down Universal Language Layer");

    // Cleanup language runtimes
    language::shutdown_runtimes().await?;

    log::info!("Universal Language Layer shutdown complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ull_initialization() {
        assert_eq!(VERSION, "1.0.0");
    }
}
