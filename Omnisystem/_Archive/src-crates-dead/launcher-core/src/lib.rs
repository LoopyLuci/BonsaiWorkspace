//! Launcher Core: Enterprise-grade kernel for application launch coordination
//!
//! This module provides the foundational infrastructure for the entire launcher system:
//! - Session management (user sessions, environment isolation)
//! - Application registry (app discovery, metadata management)
//! - Launch coordination (request queuing, dependency resolution)
//! - Lifecycle management (event hooks, state transitions)

pub mod core;
pub mod error;
pub mod session;
pub mod registry;
pub mod coordinator;
pub mod lifecycle;
pub mod types;

// Re-export main types
pub use core::LauncherCore;
pub use error::{LauncherError, LauncherResult};
pub use session::{Session, SessionManager, SessionStatus};
pub use registry::{AppMetadata, AppRegistry, AppStatus};
pub use coordinator::{LaunchRequest, LaunchCoordinator, LaunchPhase};
pub use lifecycle::{LauncherEvent, LifecycleManager};
pub use types::{AppInstance, ResourceMetrics};

#[cfg(test)]
mod tests;
