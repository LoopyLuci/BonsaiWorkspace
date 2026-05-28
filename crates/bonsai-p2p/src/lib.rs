//! Bonsai P2P transport lanes.
//!
//! Three `TransportLane` implementations for peer-to-peer transfer:
//!
//! - **`WebRtcLane`** — WebRTC DataChannel over STUN/ICE (browser-compatible,
//!   works through NAT). Feature `webrtc-lane`.
//! - **`SwarmLane`** — libp2p Kademlia swarm with noise+yamux and a
//!   request-response chunk protocol. Feature `swarm-lane`.
//! - **`OnionLane`** — Tor onion routing via `arti-client`. Anonymous, censorship-
//!   resistant transport. Feature `onion-lane`.
//!
//! All three implement [`bonsai_transfer_core::lane::TransportLane`].

#[cfg(feature = "webrtc-lane")]
pub mod webrtc;
#[cfg(feature = "swarm-lane")]
pub mod swarm;
#[cfg(feature = "onion-lane")]
pub mod onion;

#[cfg(feature = "webrtc-lane")]
pub use webrtc::WebRtcLane;
#[cfg(feature = "swarm-lane")]
pub use swarm::SwarmLane;
#[cfg(feature = "onion-lane")]
pub use onion::OnionLane;
