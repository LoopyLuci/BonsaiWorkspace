//! Unified error type wrapping every remote-desktop subsystem's error enum.

use crate::capability::TokenError;
use crate::capture::CaptureError;
use crate::encode::EncodeError;
use crate::file_transfer::FileTransferError;
use crate::input::InputError;
use crate::relay::RelayError;
use crate::rendezvous::DiscoveryError;
use crate::session::SessionError;
use crate::stream::StreamError;
use crate::telemetry::TelemetryError;
use crate::tunnel::TunnelError;

#[derive(Debug, Clone)]
pub enum Error {
    Token(String),
    Capture(String),
    Encode(String),
    FileTransfer(String),
    Input(String),
    Relay(String),
    Discovery(String),
    Session(String),
    Stream(String),
    Telemetry(String),
    Tunnel(String),
    /// Anything that doesn't fit the above.
    Other(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Token(msg) => write!(f, "capability token error: {}", msg),
            Error::Capture(msg) => write!(f, "capture error: {}", msg),
            Error::Encode(msg) => write!(f, "encode error: {}", msg),
            Error::FileTransfer(msg) => write!(f, "file transfer error: {}", msg),
            Error::Input(msg) => write!(f, "input error: {}", msg),
            Error::Relay(msg) => write!(f, "relay error: {}", msg),
            Error::Discovery(msg) => write!(f, "discovery error: {}", msg),
            Error::Session(msg) => write!(f, "session error: {}", msg),
            Error::Stream(msg) => write!(f, "stream error: {}", msg),
            Error::Telemetry(msg) => write!(f, "telemetry error: {}", msg),
            Error::Tunnel(msg) => write!(f, "tunnel error: {}", msg),
            Error::Other(msg) => write!(f, "remote-desktop error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

macro_rules! from_subsystem_error {
    ($src:ty, $variant:ident) => {
        impl From<$src> for Error {
            fn from(e: $src) -> Self {
                Error::$variant(e.to_string())
            }
        }
    };
}

from_subsystem_error!(TokenError, Token);
from_subsystem_error!(CaptureError, Capture);
from_subsystem_error!(EncodeError, Encode);
from_subsystem_error!(FileTransferError, FileTransfer);
from_subsystem_error!(InputError, Input);
from_subsystem_error!(RelayError, Relay);
from_subsystem_error!(DiscoveryError, Discovery);
from_subsystem_error!(SessionError, Session);
from_subsystem_error!(StreamError, Stream);
from_subsystem_error!(TelemetryError, Telemetry);
from_subsystem_error!(TunnelError, Tunnel);

/// Result type used by callers that want a single error type across subsystems.
pub type Result<T> = std::result::Result<T, Error>;
