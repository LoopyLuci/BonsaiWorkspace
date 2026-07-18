//! omnisystem-integration: cross-system integration primitives.
//!
//! Two related but independent stacks live here: a lightweight facade
//! ([`ModuleOrchestrator`], [`ServiceRegistry`], [`EventBus`],
//! [`HealthCheck`], [`OmnisystemConfig`]) and a more advanced
//! coordinator stack ([`coordinator::SystemCoordinator`] built on
//! [`message_transport`], [`event_system`], and [`resource_manager`]).
//! [`event_system`]/[`resource_coordinator`]/[`system_registry`] define
//! same-named types as [`event_bus`]/[`resource_manager`]/[`registry`]
//! for a different part of the stack, so only the lightweight facade is
//! glob-exported at the crate root; the rest are reachable via their
//! module paths.

pub mod command_router;
pub mod config;
pub mod coordinator;
pub mod error;
pub mod event_bus;
pub mod event_system;
pub mod health;
pub mod message_transport;
pub mod orchestrator;
pub mod registry;
pub mod resource_coordinator;
pub mod resource_manager;
pub mod sync_manager;
pub mod system_registry;

pub use config::OmnisystemConfig;
pub use error::{Error, Result};
pub use event_bus::EventBus;
pub use health::{HealthCheck, HealthStatus};
pub use orchestrator::{ModuleInstance, ModuleOrchestrator, ModuleStatus};
pub use registry::{ServiceInfo, ServiceRegistry};
