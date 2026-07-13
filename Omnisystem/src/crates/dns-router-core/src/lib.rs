pub mod error;
pub mod types;
pub mod traits;
pub mod zone;
pub mod geo_routing;
pub mod dynamic_dns;
pub mod resolver;
pub mod health;
pub mod metrics;

pub use error::{DnsError, DnsResult};
pub use types::*;
pub use traits::*;
pub use zone::DnsZone;
pub use geo_routing::GeoRouter;
pub use dynamic_dns::DynamicDnsManager;
pub use resolver::{DefaultQueryResolver, QueryCache};
pub use health::DefaultHealthChecker;
pub use metrics::DefaultMetricsCollector;
