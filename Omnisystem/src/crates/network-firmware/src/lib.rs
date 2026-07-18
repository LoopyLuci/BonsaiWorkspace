//! network-firmware: a simulated network device firmware stack.
//!
//! Covers layer 2 (switching, VLANs, MAC learning), layer 3 (IP stack,
//! ARP, routing/BGP/MPLS/OSPF), NAT, firewall/security (IDS, threat
//! signatures, anomaly detection), QoS/traffic shaping, DHCP, and an
//! SDN/telemetry network simulator. `firewall` and `security` both
//! define an independently-real `FirewallRule` for different parts of
//! the stack, so only [`types`]/[`error`] are glob-exported at the
//! crate root; everything else is reachable via its module path (as
//! the pre-written integration tests already do).

pub mod advanced_routing;
pub mod bgp;
pub mod dhcp;
pub mod error;
pub mod firewall;
pub mod layer2;
pub mod layer3;
pub mod mpls;
pub mod nat;
pub mod qos;
pub mod routing;
pub mod security;
pub mod simulation;
pub mod switching;
pub mod traffic_shaping;
pub mod types;
pub mod vlan;

pub use error::{NetworkError, Result};
pub use types::*;
