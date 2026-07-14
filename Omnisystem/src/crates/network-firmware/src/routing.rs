use crate::{Route, Result};
use dashmap::DashMap;
use std::sync::Arc;

pub struct RoutingEngine {
    routing_table: Arc<DashMap<String, Route>>,
}

impl RoutingEngine {
    pub fn new() -> Self {
        Self {
            routing_table: Arc::new(DashMap::new()),
        }
    }

    pub fn add_route(&self, route: Route) -> Result<()> {
        self.routing_table.insert(route.destination.clone(), route);
        tracing::info!("Route added");
        Ok(())
    }

    pub fn lookup_route(&self, destination: &str) -> Option<Route> {
        self.routing_table.get(destination).map(|ref_| ref_.value().clone())
    }

    pub fn get_best_route(&self, destination: &str) -> Option<Route> {
        // Match on a short prefix of the destination (bounded to the
        // string's actual length so short inputs don't panic).
        let prefix_len = destination.len().min(3);
        let prefix = &destination[..prefix_len];

        self.routing_table
            .iter()
            .filter(|ref_| ref_.value().destination.starts_with(prefix))
            .min_by_key(|ref_| ref_.value().metric)
            .map(|ref_| ref_.value().clone())
    }

    pub fn route_count(&self) -> usize {
        self.routing_table.len()
    }
}

impl Default for RoutingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_route() {
        let engine = RoutingEngine::new();
        let route = Route {
            destination: "192.168.0.0/16".to_string(),
            gateway: "192.168.1.1".to_string(),
            metric: 10,
            enabled: true,
        };
        assert!(engine.add_route(route).is_ok());
        assert_eq!(engine.route_count(), 1);
    }

    #[test]
    fn test_get_best_route_short_destination_does_not_panic() {
        let engine = RoutingEngine::new();
        engine
            .add_route(Route {
                destination: "10.0.0.0/8".to_string(),
                gateway: "10.0.0.1".to_string(),
                metric: 5,
                enabled: true,
            })
            .unwrap();

        // Destinations shorter than the 3-char prefix match used to panic
        // (an empty or matching short prefix is expected to match; a
        // non-matching short prefix should still cleanly return None
        // instead of panicking on the slice bounds).
        assert!(engine.get_best_route("").is_some());
        assert!(engine.get_best_route("1").is_some()); // "1" is a prefix of "10.0.0.0/8"
        assert!(engine.get_best_route("9").is_none());
        assert!(engine.get_best_route("10.0.0.0/8").is_some());
    }
}
