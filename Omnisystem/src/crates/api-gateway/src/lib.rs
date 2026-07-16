//! api-gateway: an in-memory API gateway route registry -- register
//! path -> service mappings ([`ApiGateway`]), and a lower-level routing
//! table ([`Router`]) for simple rule-based dispatch.

mod error;
mod gateway;
mod manager;
mod routing;
mod types;

pub use error::{Error, GatewayError, GatewayResult, Result};
pub use gateway::ApiGateway;
pub use manager::Manager;
pub use routing::Router;
pub use types::{Record, Route};
