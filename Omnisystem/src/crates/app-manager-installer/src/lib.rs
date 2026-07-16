//! App Manager Installer
//!
//! Orchestrates installing, updating, uninstalling, and rolling back
//! Omnisystem apps on top of app-manager-core's module lifecycle state
//! machine and dependency graph.

pub mod dependency_resolver;
pub mod error;
pub mod installation_context;
pub mod installer;
pub mod rollback_manager;
pub mod ull_wrapper;

pub use dependency_resolver::DependencyResolver;
pub use error::{InstallerError, Result};
pub use installation_context::InstallationContext;
pub use installer::Installer;
pub use rollback_manager::RollbackManager;
pub use ull_wrapper::register_with_ull;
