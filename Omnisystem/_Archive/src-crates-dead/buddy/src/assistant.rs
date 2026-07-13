use crate::{Result, BuddyError, CapabilityRegistry};
use dashmap::DashMap;
use std::sync::Arc;

pub struct Buddy {
    id: String,
    name: String,
    capabilities: Arc<CapabilityRegistry>,
    context: Arc<DashMap<String, String>>,
    conversation_history: Arc<std::sync::Mutex<Vec<Message>>>,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: String,
    pub timestamp: u64,
    pub user_input: String,
    pub response: String,
    pub executed_capabilities: Vec<String>,
}

impl Buddy {
    pub fn new(name: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            capabilities: Arc::new(CapabilityRegistry::new()),
            context: Arc::new(DashMap::new()),
            conversation_history: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub async fn interact(&self, user_input: String) -> Result<String> {
        tracing::info!("Buddy interacting with user input: {}", user_input);

        let response = format!("Buddy understood: {}", user_input);
        let message = Message {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            user_input,
            response: response.clone(),
            executed_capabilities: vec![],
        };

        let mut history = self.conversation_history.lock().unwrap();
        history.push(message);

        Ok(response)
    }

    pub fn register_capability(&self, name: String, description: String) -> Result<()> {
        self.capabilities.register(name, description)
    }

    pub fn list_capabilities(&self) -> Vec<(String, String)> {
        self.capabilities.list()
    }

    pub fn get_context(&self, key: &str) -> Option<String> {
        self.context.get(key).map(|ref_| ref_.value().clone())
    }

    pub fn set_context(&self, key: String, value: String) {
        self.context.insert(key, value);
    }

    pub fn conversation_length(&self) -> usize {
        self.conversation_history.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_buddy_creation() {
        let buddy = Buddy::new("Buddy".to_string());
        assert_eq!(buddy.get_name(), "Buddy");
    }

    #[tokio::test]
    async fn test_buddy_interaction() {
        let buddy = Buddy::new("Buddy".to_string());
        let response = buddy.interact("Hello".to_string()).await.unwrap();
        assert!(response.contains("Hello"));
        assert_eq!(buddy.conversation_length(), 1);
    }

    #[tokio::test]
    async fn test_context_management() {
        let buddy = Buddy::new("Buddy".to_string());
        buddy.set_context("user_name".to_string(), "Alice".to_string());
        assert_eq!(buddy.get_context("user_name"), Some("Alice".to_string()));
    }
}
