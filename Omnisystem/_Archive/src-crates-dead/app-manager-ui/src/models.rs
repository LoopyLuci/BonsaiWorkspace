//! Data models for the UI

use serde::{Deserialize, Serialize};

/// User profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: String,
    pub email: String,
    pub roles: Vec<String>,
    pub created_at: String,
}

/// App state model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub id: String,
    pub name: String,
    pub version: String,
    pub installed: bool,
    pub running: bool,
}

/// Installation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallationStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
    Uninstalled,
}

impl std::fmt::Display for InstallationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::InProgress => write!(f, "In Progress"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed(msg) => write!(f, "Failed: {}", msg),
            Self::Uninstalled => write!(f, "Uninstalled"),
        }
    }
}

/// Notification type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    Info,
    Success,
    Warning,
    Error,
}

/// UI Notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub timestamp: String,
}

impl Notification {
    pub fn info(title: &str, message: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            notification_type: NotificationType::Info,
            title: title.to_string(),
            message: message.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn success(title: &str, message: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            notification_type: NotificationType::Success,
            title: title.to_string(),
            message: message.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn warning(title: &str, message: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            notification_type: NotificationType::Warning,
            title: title.to_string(),
            message: message.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn error(title: &str, message: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            notification_type: NotificationType::Error,
            title: title.to_string(),
            message: message.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_profile() {
        let user = UserProfile {
            user_id: "user-1".to_string(),
            email: "user@example.com".to_string(),
            roles: vec!["user".to_string()],
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        assert_eq!(user.user_id, "user-1");
        assert_eq!(user.email, "user@example.com");
    }

    #[test]
    fn test_app_state() {
        let app = AppState {
            id: "app-1".to_string(),
            name: "Test App".to_string(),
            version: "1.0.0".to_string(),
            installed: true,
            running: false,
        };

        assert!(app.installed);
        assert!(!app.running);
    }

    #[test]
    fn test_installation_status_display() {
        assert_eq!(InstallationStatus::Pending.to_string(), "Pending");
        assert_eq!(InstallationStatus::InProgress.to_string(), "In Progress");
        assert_eq!(InstallationStatus::Completed.to_string(), "Completed");
        assert_eq!(InstallationStatus::Uninstalled.to_string(), "Uninstalled");
    }

    #[test]
    fn test_notification_creation() {
        let notif = Notification::success("Title", "Message");
        assert_eq!(notif.title, "Title");
        assert_eq!(notif.message, "Message");
        assert!(!notif.id.is_empty());
    }

    #[test]
    fn test_notification_types() {
        let info = Notification::info("Info", "message");
        let success = Notification::success("Success", "message");
        let warning = Notification::warning("Warning", "message");
        let error = Notification::error("Error", "message");

        assert!(!info.id.is_empty());
        assert!(!success.id.is_empty());
        assert!(!warning.id.is_empty());
        assert!(!error.id.is_empty());
    }
}
