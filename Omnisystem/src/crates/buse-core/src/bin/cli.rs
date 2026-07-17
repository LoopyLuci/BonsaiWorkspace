//! CLI for exercising the buse-core RV64I-subset interpreter: assembles a
//! tiny program (load, add, store) directly into memory and runs it.

use buse_core::{CpuState, Interpreter, MemoryBus};

fn encode_r_type(opcode: u32, rd: usize, rs1: usize, rs2: usize) -> u32 {
    opcode | ((rd as u32) << 7) | ((rs1 as u32) << 15) | ((rs2 as u32) << 20)
}

fn encode_i_type(opcode: u32, rd: usize, rs1: usize, imm: i32) -> u32 {
    opcode | ((rd as u32) << 7) | ((rs1 as u32) << 15) | (((imm as u32) & 0xFFF) << 20)
}

fn main() {
    let mut memory = MemoryBus::new(4096);

    // Program: place operands at addresses 100/104, load them, add them,
    // store the result at address 108.
    memory.write_u32(100, 19);
    memory.write_u32(104, 23);

    let mut pc = 0u32;
    let mut emit = |inst: u32| {
        memory.write_u32(pc as u64, inst);
        pc += 4;
    };
    emit(encode_i_type(0x03, 1, 0, 100)); // lw x1, 100(x0)
    emit(encode_i_type(0x03, 2, 0, 104)); // lw x2, 104(x0)
    emit(encode_r_type(0x33, 3, 1, 2)); // add x3, x1, x2

    let mut interp = Interpreter::new(CpuState::new(), memory);
    for _ in 0..3 {
        let result = interp.step();
        if let Some(exc) = result.exception {
            eprintln!("exception: {:?}", exc);
            return;
        }
    }

    println!(
        "x1={} x2={} x3={} (cycles={}, pc={})",
        interp.state().read_register(1),
        interp.state().read_register(2),
        interp.state().read_register(3),
        interp.state().cycle_count,
        interp.state().pc
    );
    assert_eq!(interp.state().read_register(3), 42);
}
