use crate::{DnsError, DnsResult, GeoLocation, GeoRoute};
use dashmap::DashMap;
use std::sync::Arc;

pub struct GeoRouter {
    routes: Arc<DashMap<String, Vec<GeoRoute>>>,
}

impl GeoRouter {
    pub fn new() -> Self {
        Self {
            routes: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_route(&self, domain: &str, route: GeoRoute) -> DnsResult<()> {
        self.routes
            .entry(domain.to_string())
            .or_insert_with(Vec::new)
            .push(route);

        Ok(())
    }

    pub async fn get_routes(&self, domain: &str) -> DnsResult<Vec<GeoRoute>> {
        self.routes
            .get(domain)
            .map(|entry| entry.clone())
            .ok_or_else(|| DnsError::GeoLocationNotFound(domain.to_string()))
    }

    pub async fn select_route(
        &self,
        domain: &str,
        location: &GeoLocation,
    ) -> DnsResult<String> {
        let routes = self.get_routes(domain).await?;

        let mut best_route = None;
        let mut best_distance = f64::MAX;

        for route in routes {
            let distance = self.haversine_distance(
                location.latitude,
                location.longitude,
                route.location.latitude,
                route.location.longitude,
            );

            if distance < best_distance {
                best_distance = distance;
                best_route = Some(route);
            }
        }

        best_route
            .map(|route| route.target)
            .ok_or_else(|| DnsError::NoHealthyServers(domain.to_string()))
    }

    pub async fn select_routes_by_country(
        &self,
        domain: &str,
        country: &str,
    ) -> DnsResult<Vec<String>> {
        let routes = self.get_routes(domain).await?;
        let targets: Vec<String> = routes
            .iter()
            .filter(|r| r.location.country == country)
            .map(|r| r.target.clone())
            .collect();

        if targets.is_empty() {
            Err(DnsError::NoHealthyServers(domain.to_string()))
        } else {
            Ok(targets)
        }
    }

    pub async fn remove_route(&self, domain: &str, target: &str) -> DnsResult<()> {
        if let Some(mut routes) = self.routes.get_mut(domain) {
            routes.retain(|r| r.target != target);
        }
        Ok(())
    }

    pub async fn update_route_priority(&self, domain: &str, target: &str, priority: u8) -> DnsResult<()> {
        if let Some(mut routes) = self.routes.get_mut(domain) {
            if let Some(route) = routes.iter_mut().find(|r| r.target == target) {
                route.priority = priority;
            }
        }
        Ok(())
    }

    fn haversine_distance(&self, lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
        const R: f64 = 6371.0;

        let lat1_rad = lat1.to_radians();
        let lat2_rad = lat2.to_radians();
        let delta_lat = (lat2 - lat1).to_radians();
        let delta_lon = (lon2 - lon1).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        R * c
    }

    pub fn route_count(&self, domain: &str) -> usize {
        self.routes
            .get(domain)
            .map(|entry| entry.len())
            .unwrap_or(0)
    }
}

impl Default for GeoRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_location(lat: f64, lon: f64, country: &str) -> GeoLocation {
        GeoLocation {
            latitude: lat,
            longitude: lon,
            country: country.to_string(),
            region: "Test Region".to_string(),
            city: "Test City".to_string(),
        }
    }

    #[tokio::test]
    async fn test_register_and_get_routes() {
        let router = GeoRouter::new();
        let location = create_test_location(37.7749, -122.4194, "US");

        let route = GeoRoute {
            location,
            target: "server-us.example.com".to_string(),
            priority: 1,
        };

        router.register_route("example.com", route).await.unwrap();
        assert_eq!(router.route_count("example.com"), 1);
    }

    #[tokio::test]
    async fn test_select_closest_route() {
        let router = GeoRouter::new();

        let us_location = create_test_location(37.7749, -122.4194, "US");
        let eu_location = create_test_location(51.5074, -0.1278, "UK");

        router
            .register_route(
                "example.com",
                GeoRoute {
                    location: us_location,
                    target: "server-us.example.com".to_string(),
                    priority: 1,
                },
            )
            .await
            .unwrap();

        router
            .register_route(
                "example.com",
                GeoRoute {
                    location: eu_location,
                    target: "server-eu.example.com".to_string(),
                    priority: 2,
                },
            )
            .await
            .unwrap();

        let query_location = create_test_location(37.7749, -122.4194, "US");
        let selected = router
            .select_route("example.com", &query_location)
            .await
            .unwrap();

        assert_eq!(selected, "server-us.example.com");
    }

    #[tokio::test]
    async fn test_select_by_country() {
        let router = GeoRouter::new();

        router
            .register_route(
                "example.com",
                GeoRoute {
                    location: create_test_location(37.7749, -122.4194, "US"),
                    target: "server-us.example.com".to_string(),
                    priority: 1,
                },
            )
            .await
            .unwrap();

        router
            .register_route(
                "example.com",
                GeoRoute {
                    location: create_test_location(51.5074, -0.1278, "UK"),
                    target: "server-uk.example.com".to_string(),
                    priority: 2,
                },
            )
            .await
            .unwrap();

        let us_servers = router
            .select_routes_by_country("example.com", "US")
            .await
            .unwrap();

        assert_eq!(us_servers.len(), 1);
        assert_eq!(us_servers[0], "server-us.example.com");
    }

    #[tokio::test]
    async fn test_remove_route() {
        let router = GeoRouter::new();
        let location = create_test_location(37.7749, -122.4194, "US");

        router
            .register_route(
                "example.com",
                GeoRoute {
                    location,
                    target: "server-us.example.com".to_string(),
                    priority: 1,
                },
            )
            .await
            .unwrap();

        assert_eq!(router.route_count("example.com"), 1);

        router
            .remove_route("example.com", "server-us.example.com")
            .await
            .unwrap();

        assert_eq!(router.route_count("example.com"), 0);
    }

    #[tokio::test]
    async fn test_update_route_priority() {
        let router = GeoRouter::new();
        let location = create_test_location(37.7749, -122.4194, "US");

        router
            .register_route(
                "example.com",
                GeoRoute {
                    location,
                    target: "server-us.example.com".to_string(),
                    priority: 1,
                },
            )
            .await
            .unwrap();

        router
            .update_route_priority("example.com", "server-us.example.com", 5)
            .await
            .unwrap();

        let routes = router.get_routes("example.com").await.unwrap();
        assert_eq!(routes[0].priority, 5);
    }
}
