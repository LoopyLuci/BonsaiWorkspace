use dashmap::DashMap;
use std::sync::Arc;

pub struct ConversationContext {
    user_profile: Arc<DashMap<String, String>>,
    session_data: Arc<DashMap<String, Vec<u8>>>,
    active: bool,
}

impl ConversationContext {
    pub fn new() -> Self {
        Self {
            user_profile: Arc::new(DashMap::new()),
            session_data: Arc::new(DashMap::new()),
            active: true,
        }
    }

    pub fn set_user_property(&self, key: String, value: String) {
        self.user_profile.insert(key, value);
    }

    pub fn get_user_property(&self, key: &str) -> Option<String> {
        self.user_profile.get(key).map(|ref_| ref_.value().clone())
    }

    pub fn store_session_data(&self, key: String, data: Vec<u8>) {
        self.session_data.insert(key, data);
    }

    pub fn retrieve_session_data(&self, key: &str) -> Option<Vec<u8>> {
        self.session_data.get(key).map(|ref_| ref_.value().clone())
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn end_session(&mut self) {
        self.active = false;
    }
}

impl Default for ConversationContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let context = ConversationContext::new();
        assert!(context.is_active());
    }

    #[test]
    fn test_user_properties() {
        let context = ConversationContext::new();
        context.set_user_property("language".to_string(), "en".to_string());
        assert_eq!(context.get_user_property("language"), Some("en".to_string()));
    }
}
