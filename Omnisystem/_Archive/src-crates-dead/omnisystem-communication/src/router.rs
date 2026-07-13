use crate::{Message, Result, CommError};
use dashmap::DashMap;
use std::sync::Arc;

pub struct MessageRouter {
    routes: Arc<DashMap<String, Vec<String>>>,
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {
            routes: Arc::new(DashMap::new()),
        }
    }

    pub fn register_route(&self, source: String, targets: Vec<String>) -> Result<()> {
        self.routes.insert(source, targets);
        Ok(())
    }

    pub fn send(&self, message: &Message) -> Result<()> {
        let _ = self.routes.get(&message.source)
            .ok_or_else(|| CommError::RouteNotFound(message.source.clone()))?;
        tracing::info!("Routed message: {}", message.id);
        Ok(())
    }

    pub fn route_count(&self) -> usize {
        self.routes.len()
    }
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_register_route() {
        let router = MessageRouter::new();
        assert!(router.register_route("s1".to_string(), vec!["t1".to_string()]).is_ok());
        assert_eq!(router.route_count(), 1);
    }
}
