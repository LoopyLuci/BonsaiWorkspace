//! omnisystem-network: RPC message framing (real JSON encode/decode).
//!
//! Note: the archived source also shipped `discovery.rs` (a
//! single-instance ServiceDiscovery) and `transport.rs` (a TCP/WebSocket
//! transport layer). Both are left out:
//! - `discovery.rs` duplicated -- with strictly less functionality
//!   (single instance per service, no health-status tracking, no load
//!   balancing) -- the already-restored `service-discovery` crate's real
//!   `ServiceRegistryImpl`/`LoadBalancer`, so wiring it in would just add
//!   an inferior second service registry.
//! - `transport.rs` was fully decorative: `open_tcp`/`open_websocket`
//!   never touched a real socket (they just minted a random UUID),
//!   `send` logged and returned `Ok(())` without sending anything, and
//!   `recv` always returned an empty `Vec` regardless of what should
//!   have arrived.

pub mod error;
pub mod protocol;

pub use error::{NetworkError, Result};
pub use protocol::{ProtocolHandler, RPCMessage};
