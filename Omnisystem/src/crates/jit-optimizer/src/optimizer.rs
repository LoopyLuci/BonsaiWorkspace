use core_ir::LairFunction;

pub struct PassManager;

impl Default for PassManager {
    fn default() -> Self { Self }
}

impl PassManager {
    pub fn run(&self, _func: &mut LairFunction) {
        // Optimization passes will be applied here
    }
}
