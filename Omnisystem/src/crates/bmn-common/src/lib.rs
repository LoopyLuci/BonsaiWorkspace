//! BMN (Broadcast Media Network) common types
//!
//! Shared frame types, source/encoder/transport traits, and metrics used
//! across the BMN capture ([`bmn-sources`](https://docs.rs/bmn-sources)),
//! compositing (`bmn-compositor`), and control (`bmcs-gateway`) crates.

pub mod encoder;
pub mod error;
pub mod frame;
pub mod metrics;
pub mod source;
pub mod transport;

pub use encoder::{EncodedPacket, Encoder, EncoderPool, EncoderStats, EncoderType};
pub use error::{BmnError, BmnResult};
pub use frame::{AudioFormat, AudioFrame, PixelFormat, VideoFrame};
pub use metrics::{MetricsCollector, StreamMetrics};
pub use source::{Capability, CapabilityToken, Source};
pub use transport::{EchoTransport, RtmpTransport, Transport, TransportProtocol, TransportStats};
