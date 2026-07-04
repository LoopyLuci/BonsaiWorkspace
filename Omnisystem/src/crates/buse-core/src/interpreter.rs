use crate::{CpuState, ExecutionResult, MemoryAccess, Exception, ExceptionCause};
use crate::memory::MemoryBus;

pub struct Interpreter {
    state: CpuState,
    memory: MemoryBus,
}

impl Interpreter {
    pub fn new(state: CpuState, memory: MemoryBus) -> Self {
        Self { state, memory }
    }

    pub fn step(&mut self) -> ExecutionResult {
        let pc = self.state.pc;
        let inst = self.memory.read_u32(pc);
        self.state.pc += 4;
        self.state.cycle_count += 1;

        let opcode = inst & 0x7F;
        match opcode {
            0x33 => {
                let rd = ((inst >> 7) & 0x1F) as usize;
                let rs1 = ((inst >> 15) & 0x1F) as usize;
                let rs2 = ((inst >> 20) & 0x1F) as usize;
                let v1 = self.state.registers.get(rs1).copied().unwrap_or(0);
                let v2 = self.state.registers.get(rs2).copied().unwrap_or(0);
                let result = v1.wrapping_add(v2);
                if rd < self.state.registers.len() {
                    self.state.registers[rd] = result;
                }
                ExecutionResult {
                    cycles: 1,
                    exception: None,
                    branch_taken: false,
                    branch_target: None,
                    memory_accesses: vec![],
                }
            }
            0x03 => {
                let rd = ((inst >> 7) & 0x1F) as usize;
                let rs1 = ((inst >> 15) & 0x1F) as usize;
                let imm = (inst as i32 >> 20) as u64;
                let addr = self.state.registers.get(rs1).copied().unwrap_or(0).wrapping_add(imm);
                let value = self.memory.read_u32(addr) as u64;
                if rd < self.state.registers.len() {
                    self.state.registers[rd] = value;
                }
                ExecutionResult {
                    cycles: 2,
                    exception: None,
                    branch_taken: false,
                    branch_target: None,
                    memory_accesses: vec![MemoryAccess {
                        address: addr,
                        size_bytes: 4,
                        is_write: false,
                        value,
                    }],
                }
            }
            _ => ExecutionResult {
                cycles: 1,
                exception: Some(Exception {
                    cause: ExceptionCause::IllegalInstruction,
                    value: inst as u64,
                }),
                branch_taken: false,
                branch_target: None,
                memory_accesses: vec![],
            },
        }
    }

    pub fn state(&self) -> &CpuState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut CpuState {
        &mut self.state
    }

    pub fn memory(&self) -> &MemoryBus {
        &self.memory
    }
}
