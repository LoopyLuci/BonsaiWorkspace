use bonsai_runtime::RuntimeManager;
use std::fs::File;
use std::io::Write;

#[tokio::test]
async fn start_clojurewasm_if_wasmtime_present() -> Result<(), Box<dyn std::error::Error>> {
    // Check for wasmtime on PATH
    let has_wasmtime = std::process::Command::new("wasmtime").arg("--version").output().is_ok();
    if !has_wasmtime {
        eprintln!("wasmtime not found; skipping wasm integration test");
        return Ok(());
    }

    // Write a minimal WAT module to a temp file
    let dir = tempdir::TempDir::new("cw_test")?;
    let path = dir.path().join("module.wat");
    let mut f = File::create(&path)?;
    let wat = r#"(module
  (func (export "_start") )
)"#;
    f.write_all(wat.as_bytes())?;
    f.flush()?;

    // Convert WAT to WASM using wat2wasm if available, otherwise call wasmtime directly on WAT
    // wasmtime accepts WAT on CLI as well.
    let module_path = path.to_string_lossy().to_string();

    let rm = RuntimeManager::new();
    let mut controller = rm.start_clojurewasm_worker(&module_path, None).await?;
    // Give it a moment to start and exit
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    // Try to wait for exit; if still running, kill
    match controller.wait().await {
        Ok(Some(code)) => eprintln!("runtime exited with code: {:?}", code),
        Ok(None) => eprintln!("runtime exited with no code"),
        Err(_) => {
            let _ = controller.kill().await;
            let _ = controller.wait().await;
        }
    }

    Ok(())
}
