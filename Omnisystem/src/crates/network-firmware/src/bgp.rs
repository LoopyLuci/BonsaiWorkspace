use dashmap::DashMap;
use std::net::IpAddr;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct BgpRoute {
    pub destination: String,
    pub next_hop: IpAddr,
    pub as_path: Vec<u32>,
    pub local_pref: u32,
}

pub struct BgpManager {
    routes: Arc<DashMap<String, BgpRoute>>,
    local_asn: u32,
}

impl BgpManager {
    pub fn new(local_asn: u32) -> Self {
        Self {
            routes: Arc::new(DashMap::new()),
            local_asn,
        }
    }

    pub fn advertise_route(&self, dest: String, route: BgpRoute) {
        self.routes.insert(dest, route);
    }

    pub fn get_route(&self, dest: &str) -> Option<BgpRoute> {
        self.routes.get(dest).map(|r| r.clone())
    }

    pub fn withdraw_route(&self, dest: &str) -> bool {
        self.routes.remove(dest).is_some()
    }

    pub fn best_path(&self) -> Option<BgpRoute> {
        self.routes
            .iter()
            .max_by_key(|entry| entry.local_pref)
            .map(|entry| entry.value().clone())
    }

    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    pub fn get_local_asn(&self) -> u32 {
        self.local_asn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bgp_route() {
        let bgp = BgpManager::new(65000);
        let route = BgpRoute {
            destination: "10.0.0.0/8".to_string(),
            next_hop: "203.0.113.1".parse().unwrap(),
            as_path: vec![65001, 65002],
            local_pref: 100,
        };
        bgp.advertise_route("10.0.0.0/8".to_string(), route);
        assert_eq!(bgp.route_count(), 1);
    }

    #[test]
    fn test_best_path() {
        let bgp = BgpManager::new(65000);
        let route1 = BgpRoute {
            destination: "10.0.0.0/8".to_string(),
            next_hop: "203.0.113.1".parse().unwrap(),
            as_path: vec![65001],
            local_pref: 100,
        };
        let route2 = BgpRoute {
            destination: "10.1.0.0/16".to_string(),
            next_hop: "203.0.113.2".parse().unwrap(),
            as_path: vec![65002],
            local_pref: 200,
        };
        bgp.advertise_route("10.0.0.0/8".to_string(), route1);
        bgp.advertise_route("10.1.0.0/16".to_string(), route2);
        assert!(bgp.best_path().is_some());
    }
}
