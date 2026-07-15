//! transfer-client: high-level client for the Bonsai TransferDaemon,
//! connecting to peers and opening bidirectional, length-delimited byte
//! streams either over a blind relay (see the sibling `relay` crate) or an
//! HTTP long-polling fallback bridge.

pub mod client;
pub mod error;
pub mod framing;
pub mod session;
pub mod stream;

pub use client::{TransferClientConfig, TransferDaemonClient};
pub use error::TransferClientError;
pub use framing::FrameCodec;
pub use session::PeerSession;
pub use stream::PeerStream;
