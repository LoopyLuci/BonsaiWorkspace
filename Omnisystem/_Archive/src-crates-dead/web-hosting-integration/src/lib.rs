pub mod error;
pub mod types;
pub mod security;
pub mod integration;
pub mod monitoring;
pub mod failover;

pub use error::{IntegrationError, IntegrationResult};
pub use types::*;
pub use security::SecurityManager;
pub use integration::ServiceIntegration;
pub use monitoring::HealthMonitor;
pub use failover::FailoverManager;
