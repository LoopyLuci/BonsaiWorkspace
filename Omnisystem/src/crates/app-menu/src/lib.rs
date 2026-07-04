//! Application Menu System - Multi-UI launcher interface
//!
//! Provides 4 complete UI implementations:
//! - Desktop (Tauri-ready with Svelte)
//! - Web (React with axum backend)
//! - CLI (clap-based + interactive mode)
//! - Client (async Rust SDK + IPC)

pub mod error;
pub mod client;
pub mod ipc;
pub mod server;
pub mod tauri;
pub mod desktop;
pub mod web;
pub mod cli;

pub use error::{AppMenuError, AppMenuResult};
pub use client::{
    LauncherClient, MockLauncherClient, UIClient, AppMetadata, AppInstance,
    LaunchRequest, LaunchResponse, SystemStatus,
};
pub use desktop::{UI as DesktopUI, DesktopConfig, AppGrid, SearchBar, StatusBar};
pub use web::{UI as WebUI, WebConfig, ReactComponents};
pub use cli::CLIInterface;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify all public types are accessible
        let _ = std::mem::size_of::<AppMenuError>();
        let _ = std::mem::size_of::<AppMetadata>();
        let _ = std::mem::size_of::<DesktopConfig>();
        let _ = std::mem::size_of::<WebConfig>();
    }
}
