//! App Manager Omnisystem Integration
//!
//! Orchestration facade over the app-manager ecosystem: wires together
//! `app-manager-repository::Repository`, `app-manager-installer::Installer`,
//! and `app-manager-core::module_lifecycle::ModuleLifecycleManager` behind a
//! single `ApplicationManager` used by `app-manager-cli` and other
//! Omnisystem consumers.

pub mod error;
pub mod manager;
pub mod ull_wrapper;

pub use error::{AppIntegrationError, Result};
pub use manager::{ApplicationManager, HealthStatus};
pub use ull_wrapper::register_with_ull;
