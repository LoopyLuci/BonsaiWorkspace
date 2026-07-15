//! Blind relay: a proof-of-work-gated TCP relay server for NAT-traversal
//! fallback in the Bonsai P2P transfer stack, plus a [`client::RelayClient`]
//! that implements `p2p_core::lane::TransportLane` so the transfer scheduler
//! can use it interchangeably with direct/DMI/Wi-Fi lanes.
//!
//! Sessions are keyed by a BLAKE3-derived [`token::RelayToken`] shared
//! out-of-band by the two peers. The server never inspects payload content
//! (it forwards opaque, already-encrypted frames), and registration is
//! rate-limited by a small proof-of-work puzzle to deter slot exhaustion.

pub mod client;
pub mod error;
pub mod server;
pub mod token;

pub use client::RelayClient;
pub use error::{RelayError, RelayResult};
pub use server::RelayServer;
pub use token::{RegisterRequest, RelayToken};
