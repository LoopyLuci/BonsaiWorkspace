//! bmn-transport: additional [`bmn_common::transport::Transport`]
//! implementations for the BMN mini-cluster (bmn-common, bmn-encoder,
//! bmn-sources, bmn-compositor, bmcs-gateway) -- MoQ and WebRTC -- plus
//! real multi-path bonding across network interfaces.

pub mod bonding;
pub mod moq;
pub mod webrtc;

pub use bonding::{MultiPathBonding, NetworkPath, PathHealth};
pub use moq::MoqTransport;
pub use webrtc::WebRTCTransport;
