//! mesh-network: a Tailscale-style mesh VPN control plane.
//!
//! [`platform::MeshPlatform`] combines mesh topology coordination
//! ([`coordination`]), Floyd-Warshall shortest-path routing with relay
//! fallback ([`mesh_routing`]), split-horizon Magic DNS ([`dns`]), and
//! DERP-like relay servers for NAT traversal ([`relay`]).

pub mod coordination;
pub mod dns;
pub mod error;
pub mod mesh_routing;
pub mod platform;
pub mod relay;

pub use error::{Error, Result};
pub use platform::{MeshConfig, MeshPlatform, NetworkHealth, PlatformStats};
