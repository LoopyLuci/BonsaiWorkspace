use anyhow::Result;
use std::path::Path;
use async_trait::async_trait;

pub struct RuntimeManager {}

#[async_trait]
pub trait RuntimeController: Send + Sync {
    fn pid(&self) -> Option<i64>;
    async fn kill(&mut self) -> Result<()>;
    async fn wait(&mut self) -> Result<Option<i32>>;
}

pub struct ProcessController {
    child: tokio::process::Child,
}

#[async_trait]
impl RuntimeController for ProcessController {
    fn pid(&self) -> Option<i64> { self.child.id().map(|p| p as i64) }
    async fn kill(&mut self) -> Result<()> { self.child.kill().await.map_err(|e| e.into()) }
    async fn wait(&mut self) -> Result<Option<i32>> { let s = self.child.wait().await?; Ok(s.code()) }
}

pub struct InProcessController {
    // Shared join handle (inside a Mutex) returning Option<i32> exit code
    join: std::sync::Arc<std::sync::Mutex<Option<tokio::task::JoinHandle<anyhow::Result<Option<i32>>>>>>,
    // We keep an interrupt handle optionally to support polite interruption (reserved)
    #[cfg(feature = "wasmtime-host")]
    #[allow(dead_code)]
    interrupt_handle: Option<Box<dyn std::any::Any + Send + Sync>>,
    #[cfg(not(feature = "wasmtime-host"))]
    #[allow(dead_code)]
    interrupt_handle: Option<Box<dyn std::any::Any + Send + Sync>>,
}

#[async_trait]
impl RuntimeController for InProcessController {
    fn pid(&self) -> Option<i64> { None }
    async fn kill(&mut self) -> Result<()> {
        // Best-effort: try polite interruption first (if available), then abort the join
        // (interrupt_handle reserved for future use)
        if let Ok(guard) = self.join.lock() {
            if let Some(h) = guard.as_ref() {
                h.abort();
            }
        }
        Ok(())
    }
    async fn wait(&mut self) -> Result<Option<i32>> {
        // Take the join handle out of the mutex and await it
        let handle_opt = {
            let mut guard = self.join.lock().unwrap();
            guard.take()
        };
        if let Some(h) = handle_opt {
            match h.await {
                Ok(res) => match res { Ok(code_opt) => Ok(code_opt), Err(e) => Err(e) },
                Err(e) => Err(e.into()),
            }
        } else {
            Ok(None)
        }
    }
}

impl RuntimeManager {
    pub fn new() -> Self { Self {} }

    const DEFAULT_SKILL_MAX_CPU_SECONDS: u64 = 30;
    const DEFAULT_SKILL_MAX_MEMORY_MB: u64 = 512;

    /// Start a Python worker by spawning the given script path with the provided port.
    /// Returns a boxed `RuntimeController` that can be used to manage the runtime.
    pub async fn start_python_worker(&self, script_path: &str, port: u16) -> Result<Box<dyn RuntimeController + Send + Sync>> {
        let mut cmd = tokio::process::Command::new(resolve_python_binary());
        cmd.arg(script_path)
            .arg(port.to_string())
            .arg("--max-cpu-seconds")
            .arg(Self::DEFAULT_SKILL_MAX_CPU_SECONDS.to_string())
            .arg("--max-memory-mb")
            .arg(Self::DEFAULT_SKILL_MAX_MEMORY_MB.to_string());
        let child = cmd.spawn()?;
        Ok(Box::new(ProcessController { child }))
    }

    /// Start a Babashka (Clojure) worker by spawning `bb` with the given script.
    pub async fn start_babashka_worker(&self, script_path: &str) -> Result<Box<dyn RuntimeController + Send + Sync>> {
        let mut cmd = tokio::process::Command::new("bb");
        cmd.arg(script_path);
        let child = cmd.spawn()?;
        Ok(Box::new(ProcessController { child }))
    }

    /// Start a ClojureWasm (Wasm/WASI) module.
    /// If the crate feature `wasmtime-host` is enabled, this will run the module in-process
    /// via the `wasmtime` crate. Otherwise it will fall back to spawning the `wasmtime` CLI.
    pub async fn start_clojurewasm_worker(&self, module_path: &str, _timeout_secs: Option<u64>) -> Result<Box<dyn RuntimeController + Send + Sync>> {
        let p = Path::new(module_path);
        if !p.exists() {
            // fall back to trying to spawn CLI which may accept WAT files too
            let mut cmd = tokio::process::Command::new("wasmtime");
            cmd.arg(module_path);
            let child = cmd.spawn()?;
            return Ok(Box::new(ProcessController { child }));
        }

        // Try in-process wasmtime if feature is enabled
        #[cfg(feature = "wasmtime-host")]
        {
            use wasmtime::*;
            use wasmtime_wasi::sync::WasiCtxBuilder;
            use std::sync::{Arc, Mutex};

            let module_path = module_path.to_string();

            // Basic engine/config for in-process execution
            let mut config = Config::new();
            config.consume_fuel(true);
            config.epoch_interruption(true);
            let engine = Engine::new(&config)?;
            let module = Module::from_file(&engine, &module_path)?;

            // Shared join handle so watchdog can abort if needed
            let join_arc: Arc<Mutex<Option<tokio::task::JoinHandle<anyhow::Result<Option<i32>>>>>> = Arc::new(Mutex::new(None));

            let engine_for_task = engine.clone();
            let module_for_task = module.clone();
            let join_arc_clone = join_arc.clone();

            let join_handle = tokio::task::spawn_blocking(move || -> anyhow::Result<Option<i32>> {
                // Build WASI context
                let wasi_ctx = WasiCtxBuilder::new().inherit_stdio().build();

                let mut store = Store::new(&engine_for_task, wasi_ctx);

                // Add some fuel budget if requested
                if let Some(sec) = _timeout_secs {
                    let _ = store.add_fuel(1_000_000_u64.saturating_mul(sec));
                } else {
                    let _ = store.add_fuel(1_000_000_u64);
                }

                let mut linker = Linker::new(&engine_for_task);
                wasmtime_wasi::add_to_linker(&mut linker, |cx: &mut _| cx).unwrap();

                let instance = linker.instantiate(&mut store, &module_for_task)?;

                // call _start if present
                if let Some(start) = instance.get_func(&mut store, "_start") {
                    let typed = start.typed::<(), ()>(&mut store)?;
                    let _ = typed.call(&mut store, ())?;
                }
                Ok(Some(0))
            });

            // store join handle in Arc<Mutex<Option<JoinHandle>>> so we can abort/wait later
            {
                let mut guard = join_arc_clone.lock().unwrap();
                *guard = Some(join_handle);
            }

            // If timeout was requested, spawn a watchdog to increment engine epoch
            if let Some(sec) = _timeout_secs {
                let engine_watch = engine.clone();
                let join_watch = join_arc.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(sec)).await;
                    // trigger epoch increment to interrupt running stores
                    engine_watch.increment_epoch();
                    // give a grace period then abort the join as last resort
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    if let Ok(guard) = join_watch.lock() {
                        if let Some(h) = guard.as_ref() {
                            h.abort();
                        }
                    }
                });
            }

            // No real PID for in-process runs
            let controller = InProcessController { join: join_arc, #[cfg(feature = "wasmtime-host")] interrupt_handle: None, #[cfg(not(feature = "wasmtime-host"))] interrupt_handle: None };
            return Ok(Box::new(controller));
        }

        // If feature is not enabled, spawn the CLI as fallback
        #[cfg(not(feature = "wasmtime-host"))]
        {
            let mut cmd = tokio::process::Command::new("wasmtime");
            cmd.arg(module_path);
            let child = cmd.spawn()?;
            return Ok(Box::new(ProcessController { child }));
        }
    }
}

fn resolve_python_binary() -> String {
    if cfg!(target_os = "windows") {
        return "python".to_string();
    }

    if which::which("python3").is_ok() {
        "python3".to_string()
    } else {
        "python".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_manager() {
        let m = RuntimeManager::new();
        let _ = m;
        assert!(true);
    }
}
