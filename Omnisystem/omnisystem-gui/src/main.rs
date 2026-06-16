// Omnisystem.exe - Complete Application Launcher with BonsaiEcosystem Integration
// Multi-language enterprise application built with TITAN + SYLVA + AETHER + AXIOM
//
// Initialization sequence:
// 1. Initialize BonsaiEcosystem (Phase 1-5: all 11 apps and services)
// 2. Launch Omnisystem App Menu (OmnisystemAppMenu.ti)
//    - Access all 11 BonsaiEcosystem applications
//    - Access 4 Omnisystem core tools
//    - Monitor 5 system services
// 3. Graceful shutdown on exit

use std::process::Command;
use std::path::PathBuf;
use std::env;

fn main() {
    let exe_dir = env::current_exe()
        .ok()
        .and_then(|pb| pb.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    // Build paths to TITAN compiler and files
    let titan_exe = exe_dir.join("Omnisystem")
        .join("titan_compiler")
        .join("target")
        .join("release")
        .join("titan.exe");

    let bonsai_startup = exe_dir.join("Omnisystem")
        .join("languages")
        .join("titan")
        .join("BonsaiEcosystemStartup.ti");

    let gui_source = exe_dir.join("Omnisystem")
        .join("languages")
        .join("titan")
        .join("OmnisystemAppMenu.ti");

    let titan_dir = gui_source.parent().unwrap();

    // Verify all components exist
    if !titan_exe.exists() {
        eprintln!("Error: TITAN compiler not found at {:?}", titan_exe);
        std::process::exit(1);
    }
    if !bonsai_startup.exists() {
        eprintln!("Error: BonsaiEcosystem startup script not found at {:?}", bonsai_startup);
        std::process::exit(1);
    }
    if !gui_source.exists() {
        eprintln!("Error: Omnisystem App Menu not found at {:?}", gui_source);
        eprintln!("Expected: OmnisystemAppMenu.ti");
        std::process::exit(1);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // PHASE 1: Initialize BonsaiEcosystem
    // ─────────────────────────────────────────────────────────────────────────────
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║                                                                ║");
    println!("║       🌿 OMNISYSTEM STARTUP WITH BONSAI ECOSYSTEM 🌿           ║");
    println!("║                                                                ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("Initializing BonsaiEcosystem...\n");

    let bonsai_result = Command::new(&titan_exe)
        .arg("run")
        .arg("BonsaiEcosystemStartup.ti")
        .current_dir(titan_dir)
        .status();

    match bonsai_result {
        Ok(status) => {
            if !status.success() {
                eprintln!("Error: BonsaiEcosystem initialization failed");
                std::process::exit(1);
            }
            println!("BonsaiEcosystem initialized successfully\n");
        }
        Err(e) => {
            eprintln!("Error: Failed to run BonsaiEcosystem startup: {}", e);
            std::process::exit(1);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // PHASE 2: Launch Omnisystem App Menu
    // ─────────────────────────────────────────────────────────────────────────────
    println!("Launching Omnisystem App Menu...\n");

    let gui_result = Command::new(&titan_exe)
        .arg("run")
        .arg("OmnisystemAppMenu.ti")
        .current_dir(titan_dir)
        .spawn();

    match gui_result {
        Ok(mut child) => {
            // Wait for GUI to exit
            let _ = child.wait();

            println!("\nGUI closed, initiating graceful shutdown...\n");

            // ─────────────────────────────────────────────────────────────────────
            // PHASE 3: Graceful Shutdown
            // ─────────────────────────────────────────────────────────────────────
            let _shutdown_result = Command::new(&titan_exe)
                .arg("run")
                .arg("BonsaiEcosystemStartup.ti")
                .arg("--shutdown")
                .current_dir(titan_dir)
                .status();

            println!("\n✅ Omnisystem shutdown complete\n");
        }
        Err(e) => {
            eprintln!("Error: Failed to launch Omnisystem GUI: {}", e);
            std::process::exit(1);
        }
    }
}
