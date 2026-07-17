//! Core CPU/machine types shared by the `buse-core` interpreter and
//! memory bus. These model a minimal RISC-V-style (RV64I subset)
//! machine state.

use serde::{Deserialize, Serialize};

/// Number of general-purpose integer registers. RISC-V defines 32
/// (x0-x31); x0 is hardwired to zero.
pub const NUM_REGISTERS: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuState {
    pub pc: u64,
    pub registers: Vec<u64>,
    pub cycle_count: u64,
}

impl CpuState {
    pub fn new() -> Self {
        Self {
            pc: 0,
            registers: vec![0; NUM_REGISTERS],
            cycle_count: 0,
        }
    }

    /// Read register `idx`. x0 always reads as zero regardless of what
    /// was last written to it, matching the RISC-V ISA convention.
    pub fn read_register(&self, idx: usize) -> u64 {
        if idx == 0 {
            return 0;
        }
        self.registers.get(idx).copied().unwrap_or(0)
    }

    /// Write `value` into register `idx`. Writes to x0 are silently
    /// discarded, matching the RISC-V ISA convention that x0 is
    /// hardwired to zero.
    pub fn write_register(&mut self, idx: usize, value: u64) {
        if idx == 0 {
            return;
        }
        if let Some(slot) = self.registers.get_mut(idx) {
            *slot = value;
        }
    }
}

impl Default for CpuState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAccess {
    pub address: u64,
    pub size_bytes: u8,
    pub is_write: bool,
    pub value: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExceptionCause {
    IllegalInstruction,
    LoadAccessFault,
    StoreAccessFault,
    Breakpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exception {
    pub cause: ExceptionCause,
    pub value: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub cycles: u64,
    pub exception: Option<Exception>,
    pub branch_taken: bool,
    pub branch_target: Option<u64>,
    pub memory_accesses: Vec<MemoryAccess>,
}
