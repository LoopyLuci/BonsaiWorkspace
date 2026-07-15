//! Error types

#[derive(Debug, Clone)]
pub enum IotError {
    DeviceNotFound(String),
    DuplicateCapability(String),
    UnsupportedProtocol(String),
    ProtocolError(String),
    SendFailed(String),
}

impl std::fmt::Display for IotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IotError::DeviceNotFound(id) => write!(f, "device not found: {id}"),
            IotError::DuplicateCapability(name) => write!(f, "duplicate capability: {name}"),
            IotError::UnsupportedProtocol(name) => write!(f, "unsupported protocol: {name}"),
            IotError::ProtocolError(msg) => write!(f, "protocol error: {msg}"),
            IotError::SendFailed(msg) => write!(f, "send failed: {msg}"),
        }
    }
}

impl std::error::Error for IotError {}

pub type Result<T> = std::result::Result<T, IotError>;
