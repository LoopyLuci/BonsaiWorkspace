//! service-manager
//!
//! Service Lifecycle Manager (SLM): manages spawn/pause/snapshot/restore of
//! service instances against a kernel adapter (mocked for Phase 2, real
//! kernel syscalls in Phase 1), backed by a manifest registry.

pub mod core;
pub mod error;
pub mod kernel_adapter;
pub mod lifecycle;
pub mod service_registry;
pub mod types;

pub use core::Core;
pub use error::{Result, SLMError};
pub use kernel_adapter::KernelAdapter;
pub use lifecycle::LifecycleManager;
pub use service_registry::ServiceRegistry;
pub use types::{
    AuditEvent, HealthStatus, ResourceQuota, ResourceUsage, SLMConfig, ServiceInstance,
    ServiceManifest, ServiceState, Snapshot,
};

/// Initialize logging for the service manager. Safe to call more than once
/// (e.g. from multiple binaries/tests in the same process); "already
/// initialized" errors are ignored.
pub fn init_logging() {
    let _ = env_logger::try_init();
}
