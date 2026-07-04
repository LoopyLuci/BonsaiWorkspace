use anyhow::{anyhow, Result};
use wasmtime::*;
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi::preview1::{self, WasiP1Ctx};

const MAX_MEMORY_BYTES: usize = 64 * 1024 * 1024;  // 64 MB
const MAX_TABLE_ELEMS:  u32   = 10_000;
const DEFAULT_FUEL:     u64   = 100_000_000;

/// Per-invocation host state living inside the `Store`. Holds the WASI preview1
/// context plus the resource limits, so `Store::limiter` can hand back a
/// long-lived `&mut StoreLimits` instead of a reference to a temporary.
struct HostState {
    wasi:   WasiP1Ctx,
    limits: StoreLimits,
}

pub struct Sandbox {
    engine: Engine,
}

impl Sandbox {
    pub fn new() -> Result<Self> {
        let mut cfg = Config::new();
        cfg.consume_fuel(true);
        cfg.wasm_memory64(false);
        let engine = Engine::new(&cfg)?;
        Ok(Self { engine })
    }

    /// Execute WASM bytes with args. Returns stdout output as String.
    pub fn run(&self, wasm_bytes: &[u8], args: Vec<String>, fuel: Option<u64>) -> Result<String> {
        let mut linker = Linker::<HostState>::new(&self.engine);
        preview1::add_to_linker_sync(&mut linker, |cx: &mut HostState| &mut cx.wasi)?;

        // Capture stdout into a pipe
        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(8192);
        let wasi: WasiP1Ctx = WasiCtxBuilder::new()
            .args(&args)
            .stdout(stdout.clone())
            .build_p1();

        let limits = StoreLimitsBuilder::new()
            .memory_size(MAX_MEMORY_BYTES)
            .table_elements(MAX_TABLE_ELEMS)
            .build();

        let mut store = Store::new(&self.engine, HostState { wasi, limits });
        store.set_fuel(fuel.unwrap_or(DEFAULT_FUEL))?;
        store.limiter(|state| &mut state.limits);

        let module   = Module::new(&self.engine, wasm_bytes)?;
        let instance = linker.instantiate(&mut store, &module)?;

        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .or_else(|_| instance.get_typed_func::<(), ()>(&mut store, "main"))?;

        match start.call(&mut store, ()) {
            Ok(_) => {}
            Err(trap) => {
                let fuel_used = fuel.unwrap_or(DEFAULT_FUEL)
                    - store.get_fuel().unwrap_or(0);
                return Err(anyhow!("WASM trap (fuel used: {}): {}", fuel_used, trap));
            }
        }

        let output = String::from_utf8_lossy(&stdout.contents()).to_string();
        Ok(output)
    }

    pub fn validate(&self, wasm_bytes: &[u8]) -> Result<()> {
        Module::new(&self.engine, wasm_bytes)?;
        Ok(())
    }
}
