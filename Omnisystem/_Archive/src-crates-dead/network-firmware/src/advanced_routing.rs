use crate::{Route, Result};
use dashmap::DashMap;
use std::sync::Arc;

pub struct AdvancedRouter {
    ospf_routes: Arc<DashMap<String, OSPFRoute>>,
    failover_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct OSPFRoute {
    pub destination: String,
    pub cost: u32,
    pub next_hop: String,
    pub area_id: u32,
}

impl AdvancedRouter {
    pub fn new(failover_enabled: bool) -> Self {
        Self {
            ospf_routes: Arc::new(DashMap::new()),
            failover_enabled,
        }
    }

    pub fn add_ospf_route(&self, route: OSPFRoute) -> Result<()> {
        self.ospf_routes.insert(route.destination.clone(), route);
        tracing::info!("OSPF route added");
        Ok(())
    }

    pub fn get_best_path(&self, destination: &str) -> Option<OSPFRoute> {
        self.ospf_routes
            .iter()
            .filter(|r| r.value().destination.starts_with(&destination[..3]))
            .min_by_key(|r| r.value().cost)
            .map(|r| r.value().clone())
    }

    pub fn route_count(&self) -> usize {
        self.ospf_routes.len()
    }
}

impl Default for AdvancedRouter {
    fn default() -> Self {
        Self::new(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advanced_router() {
        let router = AdvancedRouter::new(true);
        let route = OSPFRoute {
            destination: "192.168.1.0/24".to_string(),
            cost: 10,
            next_hop: "192.168.0.1".to_string(),
            area_id: 0,
        };
        assert!(router.add_ospf_route(route).is_ok());
        assert_eq!(router.route_count(), 1);
    }
}
