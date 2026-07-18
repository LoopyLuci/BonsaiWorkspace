//! omnisystem-kernel: a simplified microkernel simulation.
//!
//! Process/thread management, virtual memory (page tables, address
//! spaces), IPC channels, interrupt handling, device management,
//! capability-based security, a priority scheduler, and synchronization
//! primitives (spinlock, semaphore, event, barrier).

pub mod capability;
pub mod device;
pub mod error;
pub mod interrupt;
pub mod ipc;
pub mod memory;
pub mod process;
pub mod scheduling;
pub mod sync;

pub use capability::*;
pub use device::*;
pub use error::{KernelError, Result};
pub use interrupt::*;
pub use ipc::*;
pub use memory::*;
pub use process::*;
pub use scheduling::*;
pub use sync::*;
