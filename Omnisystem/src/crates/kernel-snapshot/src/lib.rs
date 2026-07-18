//! Kernel-snapshot: an in-process simulation of a "vault" snapshot/restore
//! subsystem -- tracking vault lifecycle (create/snapshot/restore/destroy),
//! capability tables, and BLAKE3-hashed serialized snapshots in a global
//! in-memory registry.

pub mod capability_table;
pub mod error;
pub mod memory;
pub mod restore;
pub mod snapshot;
pub mod syscalls;
pub mod types;

pub use capability_table::CapabilityTable;
pub use error::{Error, KernelError, Result};
pub use memory::{MemoryManager, MemoryRegion};
pub use restore::RestoreContext;
pub use snapshot::Snapshot;
pub use syscalls::{create_vault, destroy_vault, restore_vault, snapshot_vault, VaultMetadata};
pub use types::State;
