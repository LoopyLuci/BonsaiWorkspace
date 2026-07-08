//! p2p-core — Adaptive multi-path transfer engine.
//!
//! Implements TransferDaemon's core transport abstractions:
//!   - Global Sequence Numbers (GSN) — unified 64-bit sequence space across all lanes
//!   - TransportLane trait — plugin interface for DMI, TCP, relay, Bluetooth, etc.
//!   - ECF-RG scheduler — Earliest Completion First with Reorder Guard
//!   - Reassembly Window — ordered delivery with NACK-based gap detection
//!   - Retransmit Buffer — unacknowledged chunk tracking

pub mod error;
pub mod gsn;
pub mod lane;
pub mod reassembly;
pub mod scheduler;
pub mod streams;
pub mod transfer;

pub use error::{TransferError, TransferResult};
pub use gsn::GsnAllocator;
pub use lane::{LaneHealth, LaneKind, TransportLane};
pub use reassembly::{AssembledMessage, ReassemblyWindow};
pub use scheduler::{ChunkAssignment, EcfRgScheduler};
pub use transfer::{Transfer, TransferDirection, TransferHandle, TransferStatus};
