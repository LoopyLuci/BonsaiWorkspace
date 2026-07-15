//! omnisystem-connector-core: message-passing primitives (pub/sub, broadcast,
//! request/reply, streaming, and a byte-arena allocator) shared across
//! Omnisystem connectors.

pub mod arena;
pub mod broadcast;
pub mod connector;
pub mod core;
pub mod error;
pub mod manager;
pub mod message;
pub mod pubsub;
pub mod registry;
pub mod request_reply;
pub mod stream;
pub mod types;

pub use arena::{Arena, ArenaId, ArenaRef};
pub use broadcast::BroadcastConnector;
pub use connector::{Connectable, ConnectorStatus, Schema};
pub use core::Core;
pub use error::{ConnectorError, Result};
pub use manager::{Item, Manager};
pub use message::{Message, MessageEnvelope};
pub use pubsub::PubSubConnector;
pub use registry::{ConnectorMetadata, ConnectorRegistry};
pub use request_reply::RequestReplyConnector;
pub use stream::StreamConnector;
pub use types::{
    BufferingMode, CompressionMode, ConnectorConfig, ConnectorId, ConnectorType,
    DurabilityLevel, OrderingGuarantee,
};
