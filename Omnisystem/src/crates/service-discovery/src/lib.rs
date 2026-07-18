//! Service discovery: an in-memory service registry with health tracking and
//! pluggable load balancing (round-robin / random / static).

mod error;
mod load_balancer;
mod manager;
mod registry;
mod types;

pub use error::{DiscoveryError, DiscoveryResult, Error, Result};
pub use load_balancer::LoadBalancer;
pub use manager::Manager;
pub use registry::ServiceRegistryImpl;
pub use types::{
    LoadBalancingPolicy, Record, ServiceInstance, ServiceRegistry, ServiceStatus,
};
