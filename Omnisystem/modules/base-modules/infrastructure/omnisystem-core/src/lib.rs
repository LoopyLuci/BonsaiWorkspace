//! Omnisystem Core: Universal Module System
//!
//! Provides the foundation for infinite modularity, extensibility, and runtime composition.
//! Every feature in the ecosystem is a module that can be:
//! - Enabled/disabled at runtime
//! - Swapped for alternatives
//! - Composed with other modules
//! - Extended by third parties
//!
//! # Architecture
//!
//! ```text
//! Applications (VSCode, IDE, CLI, Web)
//!         ↓
//! Omnisystem Runtime (this crate)
//!         ↓
//! ┌─────────────────────────────────────┐
//! │  Module Registry & Loader           │
//! ├─────────────────────────────────────┤
//! │  ┌───────────┐  ┌───────────┐      │
//! │  │  Compiler │  │ Messaging │ ...  │
//! │  │  Module   │  │  Module   │      │
//! │  └───────────┘  └───────────┘      │
//! │      ↓               ↓              │
//! │  Capability System (enable/disable) │
//! │      ↓                              │
//! │  Data Manager (separate storage)    │
//! └─────────────────────────────────────┘
//!         ↓
//! System Services (Filesystem, Network, Process)
//! ```

pub mod module_system;
pub mod module_registry;
pub mod capability_system;
pub mod data_manager;
pub mod runtime;
pub mod error;
pub mod advanced_runtime;

pub use module_system::{OmniModule, ModuleMetadata, ModuleState};
pub use module_registry::ModuleRegistry;
pub use capability_system::{CapabilityManager, Capability};
pub use data_manager::DataManager;
pub use runtime::OmnisystemRuntime;
pub use error::{Error, Result};
pub use advanced_runtime::{
    OmnisystemRuntime as AdvancedRuntime,
    OmnisystemConfig,
    Event,
    EventStore,
    EventSourced,
    Actor,
    ActorRef,
    ActorSystem,
    WorkScheduler,
    Task,
    TaskPriority,
};

/// Omnisystem version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get comprehensive system information
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub omnisystem_version: &'static str,
    pub mode: OmniMode,
    pub active_modules: usize,
    pub total_capabilities: usize,
}

/// System operation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OmniMode {
    /// Full Omnisystem - all modules available
    OmniOS,
    /// Simplified Bonsai mode - lightweight subset
    Bonsai,
}

impl OmniMode {
    pub fn name(&self) -> &'static str {
        match self {
            OmniMode::OmniOS => "OmniOS",
            OmniMode::Bonsai => "Bonsai",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_omnisystem_modes() {
        assert_eq!(OmniMode::OmniOS.name(), "OmniOS");
        assert_eq!(OmniMode::Bonsai.name(), "Bonsai");
    }
}
