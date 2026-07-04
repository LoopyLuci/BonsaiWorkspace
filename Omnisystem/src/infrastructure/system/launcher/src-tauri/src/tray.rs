use tauri::{AppHandle, Manager};

pub fn handle_menu_item(app: &AppHandle, menu_id: &str) {
    match menu_id {
        "tray-open-launcher" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "tray-launch-workspace" => {
            // Launch Bonsai Workspace
            #[cfg(target_os = "windows")]
            std::process::Command::new("cmd")
                .args(&["/C", "bonsai-workspace --mode workspace"])
                .spawn()
                .ok();

            #[cfg(target_os = "macos")]
            std::process::Command::new("open")
                .args(&["-a", "Bonsai Workspace"])
                .spawn()
                .ok();

            #[cfg(target_os = "linux")]
            std::process::Command::new("bonsai-workspace")
                .args(&["--mode", "workspace"])
                .spawn()
                .ok();
        }
        "tray-launch-buddy" => {
            // Launch Bonsai Buddy
            #[cfg(target_os = "windows")]
            std::process::Command::new("cmd")
                .args(&["/C", "bonsai-workspace --mode buddy"])
                .spawn()
                .ok();

            #[cfg(target_os = "macos")]
            std::process::Command::new("open")
                .args(&["-a", "Bonsai Buddy"])
                .spawn()
                .ok();

            #[cfg(target_os = "linux")]
            std::process::Command::new("bonsai-workspace")
                .args(&["--mode", "buddy"])
                .spawn()
                .ok();
        }
        "tray-open-control-panel" => {
            if let Some(window) = app.get_webview_window("control-panel") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "tray-open-settings" => {
            // Open settings dialog
            println!("Opening settings...");
        }
        "tray-open-docs" => {
            // Open documentation site in default browser
            #[cfg(target_os = "windows")]
            std::process::Command::new("cmd")
                .args(&["/C", "start https://docs.bonsaiworkspace.local"])
                .spawn()
                .ok();

            #[cfg(target_os = "macos")]
            std::process::Command::new("open")
                .args(&["https://docs.bonsaiworkspace.local"])
                .spawn()
                .ok();

            #[cfg(target_os = "linux")]
            std::process::Command::new("xdg-open")
                .arg("https://docs.bonsaiworkspace.local")
                .spawn()
                .ok();
        }
        "tray-quit" => {
            std::process::exit(0);
        }
        _ => {}
    }
}
