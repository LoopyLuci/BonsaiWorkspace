//! IoT device control — protocol adapters, device registry, and the deep
//! Zigbee (Titanium) / Z-Wave (Aether) protocol stacks.
//!
//! `device`/`types` define the device and message models, `protocol` is the
//! `ProtocolHandler` adapter trait implemented by `ble`/`wifi`/`zigbee`/`zwave`/
//! `thread`, `registry`/`discovery`/`state` manage device lifecycle, and the
//! `titanium_zigbee_*`/`aether_zwave_*` modules implement the PHY/MAC/network/
//! security layers of each mesh protocol.

pub mod aether_zwave;
pub mod aether_zwave_mac;
pub mod aether_zwave_phy;
pub mod aether_zwave_routing;
pub mod aether_zwave_security;
pub mod ble;
pub mod capability;
pub mod coordination;
pub mod coordinator;
pub mod device;
pub mod discovery;
pub mod edge_compute;
pub mod error;
pub mod fallback_routing;
pub mod intelligence;
pub mod mesh_network;
pub mod multi_protocol;
pub mod multi_protocol_router;
pub mod protocol;
pub mod registry;
pub mod security;
pub mod state;
pub mod thread;
pub mod titanium_zigbee;
pub mod titanium_zigbee_aps;
pub mod titanium_zigbee_mac;
pub mod titanium_zigbee_network;
pub mod titanium_zigbee_phy;
pub mod titanium_zigbee_security;
pub mod titanium_zigbee_zcl;
pub mod transfer_daemon_bridge;
pub mod types;
pub mod wifi;
pub mod zigbee;
pub mod zwave;

pub use capability::Capability;
pub use device::{Device, DeviceState, DeviceType};
pub use error::{IotError, Result};
pub use protocol::{ProtocolHandler, ProtocolManager};
pub use registry::DeviceRegistry;
pub use types::{Message, Protocol, ProtocolConfig};
