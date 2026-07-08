pub mod error;
pub mod types;
pub mod aggregator;
pub mod integration;
pub mod distributed_systems;

pub use error::{AggregatorError, AggregatorResult};
pub use types::*;
pub use aggregator::MetricsAggregator;
pub use integration::ObservabilityIntegrationLayer;
pub use distributed_systems::{DistributedSystemsObservability, ServiceDependency, DistributedServiceGraph};
