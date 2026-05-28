//! bonsai-transfer-core — Adaptive multi-path transfer engine.
//!
//! Implements TransferDaemon's core transport abstractions:
//!   - Global Sequence Numbers (GSN) — unified 64-bit sequence space across all lanes
//!   - TransportLane trait — plugin interface for DMI, TCP, relay, Bluetooth, etc.
//!   - ECF-RG scheduler — Earliest Completion First with Reorder Guard
//!   - Reassembly Window — ordered delivery with NACK-based gap detection
//!   - Retransmit Buffer — unacknowledged chunk tracking

pub mod lane;
pub mod gsn;
pub mod scheduler;
pub mod reassembly;
pub mod transfer;
pub mod error;

pub use lane::{TransportLane, LaneHealth, LaneKind};
pub use gsn::GsnAllocator;
pub use scheduler::{EcfRgScheduler, ChunkAssignment};
pub use reassembly::{ReassemblyWindow, AssembledMessage};
pub use transfer::{Transfer, TransferHandle, TransferStatus, TransferDirection};
pub use error::{TransferError, TransferResult};
