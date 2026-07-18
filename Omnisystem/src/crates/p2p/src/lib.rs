//! Multi-transport P2P networking layer.
//!
//! Three transport lanes, each implementing the shared
//! [`p2p_core::lane::TransportLane`] trait:
//!
//! - [`SwarmLane`]: libp2p Kademlia DHT + request/response chunk transfer.
//! - [`OnionLane`]: Tor onion-routing via a local SOCKS5 proxy.
//! - [`WebRtcLane`]: WebRTC DataChannel transport (NAT-traversing, browser
//!   compatible).

pub mod onion;
pub mod swarm;
pub mod webrtc_lane;

pub use onion::OnionLane;
pub use swarm::{SwarmChunkReq, SwarmChunkResp, SwarmLane};
pub use webrtc_lane::WebRtcLane;
