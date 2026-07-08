//! Settings API endpoints

use serde::{Deserialize, Serialize};
use tauri::command;

/// User settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: String,
    pub notifications_enabled: bool,
    pub auto_update: bool,
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            notifications_enabled: true,
            auto_update: true,
            language: "en".to_string(),
        }
    }
}

/// Get user settings
#[command]
pub async fn get_settings() -> Result<Settings, String> {
    // Mock implementation
    // In production: Call GET /api/settings
    Ok(Settings::default())
}

/// Update user settings
#[command]
pub async fn update_settings(settings: Settings) -> Result<Settings, String> {
    // Validate theme
    if !["light", "dark", "auto"].contains(&settings.theme.as_str()) {
        return Err("Invalid theme".to_string());
    }

    // Validate language
    if !["en", "es", "fr", "de", "ja", "zh"].contains(&settings.language.as_str()) {
        return Err("Invalid language".to_string());
    }

    // Mock implementation
    // In production: Call PUT /api/settings
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert_eq!(settings.theme, "dark");
        assert!(settings.notifications_enabled);
        assert!(settings.auto_update);
        assert_eq!(settings.language, "en");
    }

    #[tokio::test]
    async fn test_get_settings() {
        let result = get_settings().await;
        assert!(result.is_ok());

        let settings = result.unwrap();
        assert_eq!(settings.theme, "dark");
    }

    #[tokio::test]
    async fn test_update_settings_valid() {
        let settings = Settings {
            theme: "light".to_string(),
            notifications_enabled: false,
            auto_update: false,
            language: "es".to_string(),
        };

        let result = update_settings(settings.clone()).await;
        assert!(result.is_ok());

        let updated = result.unwrap();
        assert_eq!(updated.theme, "light");
        assert_eq!(updated.language, "es");
    }

    #[tokio::test]
    async fn test_update_settings_invalid_theme() {
        let settings = Settings {
            theme: "invalid".to_string(),
            ..Default::default()
        };

        let result = update_settings(settings).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_settings_invalid_language() {
        let settings = Settings {
            language: "invalid".to_string(),
            ..Default::default()
        };

        let result = update_settings(settings).await;
        assert!(result.is_err());
    }
}
