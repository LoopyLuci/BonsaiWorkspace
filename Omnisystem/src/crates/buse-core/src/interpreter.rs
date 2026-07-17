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
            // R-type ADD (funct3/funct7 not decoded; treated as the
            // canonical RV64I "add rd, rs1, rs2").
            0x33 => {
                let rd = ((inst >> 7) & 0x1F) as usize;
                let rs1 = ((inst >> 15) & 0x1F) as usize;
                let rs2 = ((inst >> 20) & 0x1F) as usize;
                let v1 = self.state.read_register(rs1);
                let v2 = self.state.read_register(rs2);
                let result = v1.wrapping_add(v2);
                self.state.write_register(rd, result);
                ExecutionResult {
                    cycles: 1,
                    exception: None,
                    branch_taken: false,
                    branch_target: None,
                    memory_accesses: vec![],
                }
            }
            // I-type LOAD (treated as "lw rd, imm(rs1)").
            0x03 => {
                let rd = ((inst >> 7) & 0x1F) as usize;
                let rs1 = ((inst >> 15) & 0x1F) as usize;
                let imm = (inst as i32 >> 20) as u64;
                let addr = self.state.read_register(rs1).wrapping_add(imm);
                let value = self.memory.read_u32(addr) as u64;
                self.state.write_register(rd, value);
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
            // S-type STORE (treated as "sw rs2, imm(rs1)").
            0x23 => {
                let imm_lo = (inst >> 7) & 0x1F;
                let imm_hi = (inst >> 25) & 0x7F;
                let imm = (((imm_hi << 5) | imm_lo) as i32) << 20 >> 20; // sign-extend 12-bit imm
                let rs1 = ((inst >> 15) & 0x1F) as usize;
                let rs2 = ((inst >> 20) & 0x1F) as usize;
                let addr = self.state.read_register(rs1).wrapping_add(imm as i64 as u64);
                let value = self.state.read_register(rs2);
                self.memory.write_u32(addr, value as u32);
                ExecutionResult {
                    cycles: 1,
                    exception: None,
                    branch_taken: false,
                    branch_target: None,
                    memory_accesses: vec![MemoryAccess {
                        address: addr,
                        size_bytes: 4,
                        is_write: true,
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

    pub fn memory_mut(&mut self) -> &mut MemoryBus {
        &mut self.memory
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_r_type(opcode: u32, rd: usize, rs1: usize, rs2: usize) -> u32 {
        opcode | ((rd as u32) << 7) | ((rs1 as u32) << 15) | ((rs2 as u32) << 20)
    }

    fn encode_i_type(opcode: u32, rd: usize, rs1: usize, imm: i32) -> u32 {
        opcode | ((rd as u32) << 7) | ((rs1 as u32) << 15) | (((imm as u32) & 0xFFF) << 20)
    }

    fn encode_s_type(opcode: u32, rs1: usize, rs2: usize, imm: i32) -> u32 {
        let imm = imm as u32 & 0xFFF;
        let imm_lo = imm & 0x1F;
        let imm_hi = (imm >> 5) & 0x7F;
        opcode | (imm_lo << 7) | ((rs1 as u32) << 15) | ((rs2 as u32) << 20) | (imm_hi << 25)
    }

    #[test]
    fn test_add_instruction() {
        let mut interp = Interpreter::new(CpuState::new(), MemoryBus::new(4096));
        interp.state_mut().write_register(1, 10);
        interp.state_mut().write_register(2, 32);

        let inst = encode_r_type(0x33, 3, 1, 2);
        interp.memory_mut().write_u32(0, inst);

        let result = interp.step();
        assert_eq!(result.cycles, 1);
        assert!(result.exception.is_none());
        assert_eq!(interp.state().read_register(3), 42);
        assert_eq!(interp.state().pc, 4);
        assert_eq!(interp.state().cycle_count, 1);
    }

    #[test]
    fn test_add_to_x0_is_discarded() {
        let mut interp = Interpreter::new(CpuState::new(), MemoryBus::new(4096));
        interp.state_mut().write_register(1, 10);
        interp.state_mut().write_register(2, 32);

        // add x0, x1, x2 -- result must be discarded (x0 hardwired to 0).
        let inst = encode_r_type(0x33, 0, 1, 2);
        interp.memory_mut().write_u32(0, inst);

        interp.step();
        assert_eq!(interp.state().read_register(0), 0);
    }

    #[test]
    fn test_load_instruction() {
        let mut interp = Interpreter::new(CpuState::new(), MemoryBus::new(4096));
        interp.memory_mut().write_u32(100, 0xDEADBEEF);
        interp.state_mut().write_register(1, 100); // base address in rs1

        let inst = encode_i_type(0x03, 5, 1, 0);
        interp.memory_mut().write_u32(0, inst);

        let result = interp.step();
        assert_eq!(interp.state().read_register(5), 0xDEADBEEF);
        assert_eq!(result.memory_accesses.len(), 1);
        assert!(!result.memory_accesses[0].is_write);
        assert_eq!(result.memory_accesses[0].address, 100);
    }

    #[test]
    fn test_store_then_load_roundtrip() {
        let mut interp = Interpreter::new(CpuState::new(), MemoryBus::new(4096));
        interp.state_mut().write_register(1, 200); // base address
        interp.state_mut().write_register(2, 0xCAFEBABE); // value to store

        let store_inst = encode_s_type(0x23, 1, 2, 0);
        interp.memory_mut().write_u32(0, store_inst);
        let store_result = interp.step();
        assert!(store_result.memory_accesses[0].is_write);
        assert_eq!(store_result.memory_accesses[0].value, 0xCAFEBABE);

        // Now load it back with a fresh instruction at pc=4.
        let load_inst = encode_i_type(0x03, 3, 1, 0);
        interp.memory_mut().write_u32(4, load_inst);
        interp.step();
        assert_eq!(interp.state().read_register(3), 0xCAFEBABE);
    }

    #[test]
    fn test_illegal_instruction_raises_exception() {
        let mut interp = Interpreter::new(CpuState::new(), MemoryBus::new(4096));
        // opcode 0x7F is not handled by this minimal decoder.
        interp.memory_mut().write_u32(0, 0x7F);

        let result = interp.step();
        assert!(result.exception.is_some());
        assert_eq!(result.exception.unwrap().cause, ExceptionCause::IllegalInstruction);
    }
}
