use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Color definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Color {
    pub hex: String,
    pub rgb: (u8, u8, u8),
}

impl Color {
    pub fn new(hex: String, rgb: (u8, u8, u8)) -> Self {
        Color { hex, rgb }
    }
}

/// Theme definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub theme_id: String,
    pub primary_color: Color,
    pub secondary_color: Color,
    pub background_color: Color,
    pub text_color: Color,
    pub success_color: Color,
    pub error_color: Color,
    pub warning_color: Color,
}

impl Theme {
    pub fn light() -> Self {
        Theme {
            name: "Light".to_string(),
            theme_id: "light-default".to_string(),
            primary_color: Color::new("#0066CC".to_string(), (0, 102, 204)),
            secondary_color: Color::new("#6B7280".to_string(), (107, 114, 128)),
            background_color: Color::new("#FFFFFF".to_string(), (255, 255, 255)),
            text_color: Color::new("#111827".to_string(), (17, 24, 39)),
            success_color: Color::new("#10B981".to_string(), (16, 185, 129)),
            error_color: Color::new("#EF4444".to_string(), (239, 68, 68)),
            warning_color: Color::new("#F59E0B".to_string(), (245, 158, 11)),
        }
    }

    pub fn dark() -> Self {
        Theme {
            name: "Dark".to_string(),
            theme_id: "dark-default".to_string(),
            primary_color: Color::new("#3B82F6".to_string(), (59, 130, 246)),
            secondary_color: Color::new("#9CA3AF".to_string(), (156, 163, 175)),
            background_color: Color::new("#1F2937".to_string(), (31, 41, 55)),
            text_color: Color::new("#F3F4F6".to_string(), (243, 244, 246)),
            success_color: Color::new("#34D399".to_string(), (52, 211, 153)),
            error_color: Color::new("#F87171".to_string(), (248, 113, 113)),
            warning_color: Color::new("#FBBF24".to_string(), (251, 191, 36)),
        }
    }

    pub fn as_css(&self) -> String {
        format!(
            r#":root {{
  --primary-color: {};
  --secondary-color: {};
  --background-color: {};
  --text-color: {};
  --success-color: {};
  --error-color: {};
  --warning-color: {};
}}"#,
            self.primary_color.hex,
            self.secondary_color.hex,
            self.background_color.hex,
            self.text_color.hex,
            self.success_color.hex,
            self.error_color.hex,
            self.warning_color.hex
        )
    }
}

/// Theme manager
pub struct ThemeManager {
    themes: Arc<DashMap<String, Theme>>,
    active_theme: Arc<std::sync::Mutex<String>>,
}

impl ThemeManager {
    pub fn new() -> Self {
        let manager = ThemeManager {
            themes: Arc::new(DashMap::new()),
            active_theme: Arc::new(std::sync::Mutex::new("light-default".to_string())),
        };

        // Register default themes
        manager.themes.insert("light-default".to_string(), Theme::light());
        manager.themes.insert("dark-default".to_string(), Theme::dark());

        manager
    }

    pub fn register_theme(&self, theme: Theme) {
        tracing::debug!("ThemeManager: Registering theme '{}'", theme.name);
        self.themes.insert(theme.theme_id.clone(), theme);
    }

    pub fn set_active_theme(&self, theme_id: String) -> bool {
        if self.themes.contains_key(&theme_id) {
            *self.active_theme.lock().unwrap() = theme_id;
            true
        } else {
            false
        }
    }

    pub fn get_active_theme(&self) -> Option<Theme> {
        let theme_id = self.active_theme.lock().unwrap().clone();
        self.themes.get(&theme_id).map(|t| t.clone())
    }

    pub fn get_theme(&self, theme_id: &str) -> Option<Theme> {
        self.themes.get(theme_id).map(|t| t.clone())
    }

    pub fn list_themes(&self) -> Vec<Theme> {
        self.themes.iter().map(|t| t.value().clone()).collect()
    }

    pub fn theme_count(&self) -> usize {
        self.themes.len()
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_light_theme() {
        let theme = Theme::light();
        assert_eq!(theme.name, "Light");
        assert_eq!(theme.background_color.hex, "#FFFFFF");
    }

    #[test]
    fn test_dark_theme() {
        let theme = Theme::dark();
        assert_eq!(theme.name, "Dark");
        assert_eq!(theme.background_color.hex, "#1F2937");
    }

    #[test]
    fn test_theme_css() {
        let theme = Theme::light();
        let css = theme.as_css();
        assert!(css.contains("--primary-color"));
    }

    #[test]
    fn test_theme_manager() {
        let manager = ThemeManager::new();
        assert_eq!(manager.theme_count(), 2);
    }

    #[test]
    fn test_math() {
        assert_eq!(2 + 2, 4);
    }
}
