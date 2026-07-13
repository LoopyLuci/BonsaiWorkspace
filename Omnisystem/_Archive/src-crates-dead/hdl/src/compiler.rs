use crate::DeviceDefinition;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub fn compile(def: &DeviceDefinition, output_dir: &str) -> Result<()> {
    let path = Path::new(output_dir).join(format!("{}.rs", def.name.to_lowercase()));
    let code = generate_device_code(def);
    fs::write(&path, code)?;
    Ok(())
}

fn generate_device_code(def: &DeviceDefinition) -> String {
    let name = &def.name;
    let reg_count = def.registers.iter().map(|r| r.count as usize).sum::<usize>();
    let mem_size = def.memory.ram.iter().map(|r| r.size_bytes).sum::<u64>();
    let stages = format!("{:?}", &def.pipeline.stages);

    format!(
        r#"// Auto-generated device model for {name}
// ISA: {isa}, Architecture: {arch}

use buse_core::{{Device, CpuState, MemoryBus, ExecutionResult}};

pub struct {name}Device {{
    pub state: CpuState,
    pub memory: MemoryBus,
}}

impl {name}Device {{
    pub fn new() -> Self {{
        Self {{
            state: CpuState::with_registers({reg_count}),
            memory: MemoryBus::with_ram({mem_size}),
        }}
    }}
}}

impl Device for {name}Device {{
    fn step(&mut self) -> ExecutionResult {{
        let _inst = self.memory.read_u32(self.state.pc);
        self.state.pc += 4;
        self.state.cycle_count += 1;

        ExecutionResult {{
            cycles: 1,
            exception: None,
            branch_taken: false,
            branch_target: None,
            memory_accesses: vec![],
        }}
    }}

    fn reset(&mut self) {{
        self.state.reset();
        self.memory.clear();
    }}

    fn read_register(&self, index: usize) -> u64 {{
        if index < self.state.registers.len() {{
            self.state.registers[index]
        }} else {{
            0
        }}
    }}

    fn write_register(&mut self, index: usize, value: u64) {{
        if index < self.state.registers.len() {{
            self.state.registers[index] = value;
        }}
    }}

    fn get_pc(&self) -> u64 {{
        self.state.pc
    }}

    fn set_pc(&mut self, pc: u64) {{
        self.state.pc = pc;
    }}

    fn get_state(&self) -> &CpuState {{
        &self.state
    }}

    fn get_state_mut(&mut self) -> &mut CpuState {{
        &mut self.state
    }}
}}
"#,
        name = name,
        isa = def.isa,
        arch = def.arch,
        reg_count = reg_count,
        mem_size = mem_size,
    )
}
