//! Application state management

use crate::models::{UserProfile, Notification};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Application state
pub struct AppState {
    /// Current logged-in user
    pub user: Mutex<Option<UserProfile>>,
    /// Authentication token
    pub token: Mutex<Option<String>>,
    /// Notifications
    pub notifications: Mutex<Vec<Notification>>,
    /// Cached app data
    pub cache: Mutex<HashMap<String, String>>,
}

impl AppState {
    /// Create new application state
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            user: Mutex::new(None),
            token: Mutex::new(None),
            notifications: Mutex::new(Vec::new()),
            cache: Mutex::new(HashMap::new()),
        })
    }

    /// Set current user
    pub fn set_user(&self, user: UserProfile) {
        let mut current = self.user.lock().unwrap();
        *current = Some(user);
    }

    /// Get current user
    pub fn get_user(&self) -> Option<UserProfile> {
        self.user.lock().unwrap().clone()
    }

    /// Clear user (logout)
    pub fn clear_user(&self) {
        let mut current = self.user.lock().unwrap();
        *current = None;
    }

    /// Set authentication token
    pub fn set_token(&self, token: String) {
        let mut current = self.token.lock().unwrap();
        *current = Some(token);
    }

    /// Get authentication token
    pub fn get_token(&self) -> Option<String> {
        self.token.lock().unwrap().clone()
    }

    /// Clear token (logout)
    pub fn clear_token(&self) {
        let mut current = self.token.lock().unwrap();
        *current = None;
    }

    /// Check if user is logged in
    pub fn is_logged_in(&self) -> bool {
        self.get_user().is_some() && self.get_token().is_some()
    }

    /// Add notification
    pub fn add_notification(&self, notification: Notification) {
        let mut notifs = self.notifications.lock().unwrap();
        notifs.push(notification);
    }

    /// Get all notifications
    pub fn get_notifications(&self) -> Vec<Notification> {
        self.notifications.lock().unwrap().clone()
    }

    /// Clear all notifications
    pub fn clear_notifications(&self) {
        let mut notifs = self.notifications.lock().unwrap();
        notifs.clear();
    }

    /// Set cache value
    pub fn set_cache(&self, key: &str, value: String) {
        let mut cache = self.cache.lock().unwrap();
        cache.insert(key.to_string(), value);
    }

    /// Get cache value
    pub fn get_cache(&self, key: &str) -> Option<String> {
        self.cache.lock().unwrap().get(key).cloned()
    }

    /// Clear cache
    pub fn clear_cache(&self) {
        self.cache.lock().unwrap().clear();
    }
}

// Note: AppState is typically used as Arc<Self> via AppState::new()
// Default trait not implemented as it would lose the Arc wrapper

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_creation() {
        let state = AppState::new();
        assert!(!state.is_logged_in());
    }

    #[test]
    fn test_user_management() {
        let state = AppState::new();
        let user = UserProfile {
            user_id: "user-1".to_string(),
            email: "user@example.com".to_string(),
            roles: vec!["user".to_string()],
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        state.set_user(user.clone());
        let retrieved = state.get_user();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().user_id, "user-1");

        state.clear_user();
        assert!(state.get_user().is_none());
    }

    #[test]
    fn test_token_management() {
        let state = AppState::new();
        assert!(state.get_token().is_none());

        state.set_token("test-token".to_string());
        assert_eq!(state.get_token(), Some("test-token".to_string()));

        state.clear_token();
        assert!(state.get_token().is_none());
    }

    #[test]
    fn test_login_status() {
        let state = AppState::new();
        assert!(!state.is_logged_in());

        let user = UserProfile {
            user_id: "user-1".to_string(),
            email: "user@example.com".to_string(),
            roles: vec!["user".to_string()],
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        state.set_user(user);
        state.set_token("token".to_string());
        assert!(state.is_logged_in());

        state.clear_user();
        assert!(!state.is_logged_in());
    }

    #[test]
    fn test_notifications() {
        let state = AppState::new();
        assert!(state.get_notifications().is_empty());

        let notif = Notification::success("Title", "Message");
        state.add_notification(notif);
        assert_eq!(state.get_notifications().len(), 1);

        state.clear_notifications();
        assert!(state.get_notifications().is_empty());
    }

    #[test]
    fn test_cache_management() {
        let state = AppState::new();
        assert!(state.get_cache("key").is_none());

        state.set_cache("key", "value".to_string());
        assert_eq!(state.get_cache("key"), Some("value".to_string()));

        state.clear_cache();
        assert!(state.get_cache("key").is_none());
    }
}
