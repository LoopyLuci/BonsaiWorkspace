use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub project_root: Option<PathBuf>,
    pub theme: Theme,
    pub font_size: u16,
    pub auto_save: bool,
    pub auto_compile: bool,
    pub show_ast: bool,
    pub show_mir: bool,
    pub enable_linting: bool,
    pub syntax_highlighting: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Theme {
    Dark,
    Light,
    HighContrast,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            project_root: None,
            theme: Theme::Dark,
            font_size: 12,
            auto_save: true,
            auto_compile: false,
            show_ast: false,
            show_mir: false,
            enable_linting: true,
            syntax_highlighting: true,
        }
    }
}

impl Settings {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_dir = dirs::config_dir()
            .ok_or("Could not find config directory")?
            .join("bonsai-compiler-gui");

        std::fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("settings.json");
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Settings::default())
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_dir = dirs::config_dir()
            .ok_or("Could not find config directory")?
            .join("bonsai-compiler-gui");

        std::fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("settings.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;

        Ok(())
    }
}
