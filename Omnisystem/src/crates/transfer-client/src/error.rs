//! Error types for the TransferDaemon client.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransferClientError {
    #[error("transport unavailable: {0}")]
    TransportUnavailable(String),
    #[error("configuration error: {0}")]
    ConfigError(String),
    #[error("failed to connect to peer {peer}: {reason}")]
    ConnectionFailed { peer: String, reason: String },
    #[error("failed to send on stream {name}: {reason}")]
    SendError { name: String, reason: String },
    #[error("failed to receive on stream {name}: {reason}")]
    ReceiveError { name: String, reason: String },
}
