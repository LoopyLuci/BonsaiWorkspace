//! Remote desktop: zero-trust capability tokens, peer discovery, session
//! management, adaptive streaming, input injection, file transfer, and
//! TCP tunneling.

pub mod capability;
pub mod capture;
pub mod core;
pub mod encode;
pub mod error;
pub mod file_transfer;
pub mod input;
pub mod relay;
pub mod rendezvous;
pub mod session;
pub mod stream;
pub mod telemetry;
pub mod tunnel;
pub mod types;

pub use capability::{Capability, RemoteDesktopToken, RevocationStatus, TokenError};
pub use core::Core;
pub use error::{Error, Result};
pub use types::State;

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a remote desktop session.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(uuid::Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SessionId({})", self.0)
    }
}

/// Unique identifier for a remote peer (an Ed25519-sized public key or
/// equivalent 32-byte handle).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId([u8; 32]);

impl PeerId {
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self(*bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl fmt::Debug for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PeerId({}...)", &hex::encode(self.0)[..8])
    }
}

/// Point-in-time network/quality statistics for a stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStats {
    pub bitrate_mbps: f64,
    pub rtt_ms: f64,
    pub packet_loss_percent: f64,
    pub fps: f64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub last_update: chrono::DateTime<chrono::Utc>,
}
