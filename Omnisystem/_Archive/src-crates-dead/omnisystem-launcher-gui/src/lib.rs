// Omnisystem Launcher GUI
// Native desktop application for launching and managing applications
// Built with Tauri and Svelte

pub mod gui {
    pub fn info() -> &'static str {
        "Omnisystem Launcher GUI v1.0.0 - Native desktop application"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gui_info() {
        assert!(gui::info().contains("Launcher"));
    }
}
