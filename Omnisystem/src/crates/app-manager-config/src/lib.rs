//! App Manager Config
//!
//! Manages per-app configuration (resources, environment, logging,
//! restart policy) for the Omnisystem app management ecosystem: an
//! in-memory config store with JSON/TOML (de)serialization, and a
//! separate per-app environment variable manager.

pub mod app_config;
pub mod config_manager;
pub mod environment;
pub mod error;
pub mod ull_wrapper;

pub use app_config::{AppConfig, LogLevel, LogOutput, ProcessPriority, ResourceAllocation};
pub use config_manager::ConfigManager;
pub use environment::EnvironmentManager;
pub use error::{ConfigError, Result};
pub use ull_wrapper::register_with_ull;
