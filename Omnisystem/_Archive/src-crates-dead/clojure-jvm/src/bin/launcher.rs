//! Clojure JVM Launcher Binary
//!
//! This is the Titanium launcher that starts the JVM with capability enforcement

use clojure_jvm::{init, Capability, RuntimeConfig};
use std::env;
use log::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Clojure JVM Launcher v1.0.0");

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <clojure-file> [args...]", args[0]);
        eprintln!("\nExample: {} examples/hello.clj", args[0]);
        std::process::exit(1);
    }

    let clojure_file = &args[1];
    let extra_args = &args[2..];

    info!("Loading Clojure file: {}", clojure_file);

    // Build runtime configuration
    let config = RuntimeConfig::default()
        // Allow access to current directory and /tmp
        .with_capability(Capability::Filesystem(vec![
            ".".to_string(),
            "/tmp".to_string(),
        ]))
        // Allow network access
        .with_capability(Capability::Network)
        // Allow threading
        .with_capability(Capability::Threading)
        // Set heap size
        .with_heap_size(512)
        // Set timeout to 5 minutes
        .with_timeout(300);

    // Create runtime
    let runtime = init(config)?;

    info!("Starting JVM...");
    runtime.start()?;

    info!("JVM started successfully");

    // Load and execute Clojure file
    match std::fs::read_to_string(clojure_file) {
        Ok(code) => {
            info!("Executing Clojure code ({} bytes)", code.len());

            match runtime.eval(&code) {
                Ok(result) => {
                    println!("{}", result);
                    info!("Execution completed successfully");
                }
                Err(e) => {
                    eprintln!("Error executing Clojure code: {}", e);
                    runtime.stop()?;
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            runtime.stop()?;
            std::process::exit(1);
        }
    }

    // Cleanup
    runtime.stop()?;

    info!("Launcher exiting");
    Ok(())
}
