use crate::{GatewayError, GatewayResult, Route};
use dashmap::DashMap;
use std::sync::Arc;

pub struct ApiGateway {
    routes: Arc<DashMap<String, Route>>,
}

impl ApiGateway {
    pub fn new() -> Self {
        Self {
            routes: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_route(&self, route: &Route) -> GatewayResult<()> {
        self.routes.insert(route.path.clone(), route.clone());
        Ok(())
    }

    pub async fn get_route(&self, path: &str) -> GatewayResult<Route> {
        self.routes
            .get(path)
            .map(|entry| entry.clone())
            .ok_or(GatewayError::RouteNotFound)
    }

    pub fn route_count(&self) -> usize {
        self.routes.len()
    }
}

impl Default for ApiGateway {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_route() {
        let gateway = ApiGateway::new();
        let route = Route {
            path: "/api/users".to_string(),
            target_service: "user-service".to_string(),
            methods: vec!["GET".to_string(), "POST".to_string()],
        };

        gateway.register_route(&route).await.unwrap();
        assert_eq!(gateway.route_count(), 1);
    }

    #[tokio::test]
    async fn test_get_route() {
        let gateway = ApiGateway::new();
        let route = Route {
            path: "/api/users".to_string(),
            target_service: "user-service".to_string(),
            methods: vec!["GET".to_string()],
        };

        gateway.register_route(&route).await.unwrap();
        let retrieved = gateway.get_route("/api/users").await.unwrap();
        assert_eq!(retrieved.target_service, "user-service");
    }
}
