//! buse-core: a minimal RV64I-subset CPU interpreter with a byte-addressed
//! memory bus and pluggable MMIO devices.

pub mod interpreter;
pub mod memory;
pub mod types;

pub use interpreter::Interpreter;
pub use memory::{MemoryBus, MmioDevice};
pub use types::{CpuState, ExecutionResult, Exception, ExceptionCause, MemoryAccess, NUM_REGISTERS};
