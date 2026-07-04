mod error;
mod types;
mod router;

pub use error::{TrafficError, TrafficResult};
pub use types::{RoutingPolicy, RoutingStrategy, WeightedDestination, CanaryDeployment, DeploymentStatus, TrafficShapingPolicy, RouteConfig};
pub use router::TrafficRouter;
