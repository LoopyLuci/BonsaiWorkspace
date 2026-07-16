use crate::{GatewayError, GatewayResult};
use dashmap::DashMap;
use std::sync::Arc;

pub struct Router {
    rules: Arc<DashMap<String, String>>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(DashMap::new()),
        }
    }

    pub async fn route(&self, path: &str) -> GatewayResult<String> {
        self.rules
            .get(path)
            .map(|entry| entry.value().clone())
            .ok_or(GatewayError::RouteNotFound)
    }

    pub async fn add_rule(&self, path: &str, target: &str) -> GatewayResult<()> {
        self.rules.insert(path.to_string(), target.to_string());
        Ok(())
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_rule() {
        let router = Router::new();
        router.add_rule("/api/users", "user-service").await.unwrap();
        assert_eq!(router.rule_count(), 1);
    }

    #[tokio::test]
    async fn test_route() {
        let router = Router::new();
        router.add_rule("/api/users", "user-service").await.unwrap();
        let target = router.route("/api/users").await.unwrap();
        assert_eq!(target, "user-service");
    }
}
