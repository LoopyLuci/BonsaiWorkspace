// Omnisystem.exe - GUI Launcher
// Multi-language enterprise application built with TITAN + SYLVA + AETHER + AXIOM

use std::process::Command;
use std::path::PathBuf;
use std::env;

fn main() {
    let exe_dir = env::current_exe()
        .ok()
        .and_then(|pb| pb.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    // Build paths to TITAN compiler and GUI source
    let titan_exe = exe_dir.join("Omnisystem")
        .join("titan_compiler")
        .join("target")
        .join("release")
        .join("titan.exe");

    let gui_source = exe_dir.join("Omnisystem")
        .join("languages")
        .join("titan")
        .join("OmnisystemGUI_v2.ti");

    // Launch TITAN GUI
    if titan_exe.exists() && gui_source.exists() {
        let titan_dir = gui_source.parent().unwrap();

        Command::new(&titan_exe)
            .arg("run")
            .arg("OmnisystemGUI_v2.ti")
            .current_dir(titan_dir)
            .spawn()
            .expect("Failed to launch Omnisystem GUI");
    } else {
        eprintln!("Error: Omnisystem GUI components not found");
        eprintln!("TITAN: {:?}", titan_exe);
        eprintln!("GUI: {:?}", gui_source);
        std::process::exit(1);
    }
}
