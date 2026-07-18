use crate::{RoutingPolicy, RoutingStrategy, WeightedDestination, TrafficShapingPolicy, TrafficError, TrafficResult, CanaryDeployment, DeploymentStatus};
use dashmap::DashMap;
use rand::Rng;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use uuid::Uuid;

pub struct TrafficRouter {
    policies: Arc<DashMap<String, RoutingPolicy>>,
    weighted_destinations: Arc<DashMap<Uuid, WeightedDestination>>,
    shaping_policies: Arc<DashMap<String, TrafficShapingPolicy>>,
    canary_deployments: Arc<DashMap<Uuid, CanaryDeployment>>,
    /// Per-service round-robin cursor.
    round_robin_cursors: Arc<DashMap<String, AtomicU32>>,
    /// Per-destination active-connection counters, used by the
    /// LeastConnections strategy.
    connection_counts: Arc<DashMap<Uuid, AtomicU64>>,
}

impl TrafficRouter {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(DashMap::new()),
            weighted_destinations: Arc::new(DashMap::new()),
            shaping_policies: Arc::new(DashMap::new()),
            canary_deployments: Arc::new(DashMap::new()),
            round_robin_cursors: Arc::new(DashMap::new()),
            connection_counts: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_routing_policy(&self, policy: &RoutingPolicy) -> TrafficResult<()> {
        self.policies.insert(policy.service_name.clone(), policy.clone());
        Ok(())
    }

    /// Select a real destination for `service_name` according to its
    /// registered routing strategy, rather than just echoing the
    /// strategy's name back. Returns the chosen destination's version
    /// (formatted "service:version") and tracks an active connection for
    /// LeastConnections accounting.
    pub async fn route_request(&self, service_name: &str) -> TrafficResult<String> {
        let policy = self.policies.get(service_name).ok_or(TrafficError::PolicyNotFound)?;
        let strategy = policy.routing_strategy.clone();
        drop(policy);

        let destinations = self.get_weighted_destinations(service_name).await?;
        if destinations.is_empty() {
            return Err(TrafficError::NoDestinationsAvailable);
        }

        let chosen = match strategy {
            RoutingStrategy::RoundRobin => {
                let cursor = self.round_robin_cursors.entry(service_name.to_string()).or_insert_with(|| AtomicU32::new(0));
                let idx = cursor.fetch_add(1, Ordering::SeqCst) as usize % destinations.len();
                destinations[idx].clone()
            }
            RoutingStrategy::Random => {
                let idx = rand::thread_rng().gen_range(0..destinations.len());
                destinations[idx].clone()
            }
            RoutingStrategy::WeightedDistribution => {
                let total_weight: u32 = destinations.iter().map(|d| d.weight).sum();
                if total_weight == 0 {
                    destinations[0].clone()
                } else {
                    let mut roll = rand::thread_rng().gen_range(0..total_weight);
                    let mut picked = destinations[0].clone();
                    for dest in &destinations {
                        if roll < dest.weight {
                            picked = dest.clone();
                            break;
                        }
                        roll -= dest.weight;
                    }
                    picked
                }
            }
            RoutingStrategy::LeastConnections => destinations
                .iter()
                .min_by_key(|d| self.connection_counts.get(&d.destination_id).map(|c| c.load(Ordering::SeqCst)).unwrap_or(0))
                .cloned()
                .unwrap_or_else(|| destinations[0].clone()),
        };

        self.connection_counts
            .entry(chosen.destination_id)
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::SeqCst);

        Ok(format!("{}:{}", chosen.service_name, chosen.version))
    }

    /// Release a connection previously counted by `route_request`, so
    /// LeastConnections reflects requests actually finishing rather than
    /// growing without bound.
    pub async fn release_connection(&self, destination_id: Uuid) {
        if let Some(counter) = self.connection_counts.get(&destination_id) {
            let _ = counter.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |c| Some(c.saturating_sub(1)));
        }
    }

    pub async fn add_weighted_destination(&self, dest: &WeightedDestination) -> TrafficResult<()> {
        if dest.weight > 100 {
            return Err(TrafficError::InvalidWeight);
        }

        self.weighted_destinations.insert(dest.destination_id, dest.clone());
        Ok(())
    }

    pub async fn get_weighted_destinations(&self, service_name: &str) -> TrafficResult<Vec<WeightedDestination>> {
        let mut destinations = Vec::new();

        for entry in self.weighted_destinations.iter() {
            if entry.value().service_name == service_name {
                destinations.push(entry.value().clone());
            }
        }

        Ok(destinations)
    }

    pub async fn register_traffic_shaping(&self, policy: &TrafficShapingPolicy) -> TrafficResult<()> {
        self.shaping_policies.insert(policy.service_name.clone(), policy.clone());
        Ok(())
    }

    pub async fn start_canary_deployment(&self, deployment: &CanaryDeployment) -> TrafficResult<()> {
        if deployment.canary_traffic_percent > 100 {
            return Err(TrafficError::InvalidWeight);
        }

        self.canary_deployments.insert(deployment.deployment_id, deployment.clone());
        Ok(())
    }

    pub async fn get_canary_deployment(&self, deployment_id: Uuid) -> TrafficResult<CanaryDeployment> {
        self.canary_deployments
            .get(&deployment_id)
            .map(|d| d.clone())
            .ok_or(TrafficError::CanaryDeploymentFailed)
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }
}

impl Default for TrafficRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_routing_policy() {
        let router = TrafficRouter::new();
        let policy = RoutingPolicy {
            policy_id: Uuid::new_v4(),
            service_name: "api".to_string(),
            routing_strategy: RoutingStrategy::RoundRobin,
            timeout_ms: 5000,
            retries: 3,
        };

        router.register_routing_policy(&policy).await.unwrap();
        assert_eq!(router.policy_count(), 1);
    }

    #[tokio::test]
    async fn test_route_request_round_robin_cycles_destinations() {
        let router = TrafficRouter::new();
        router
            .register_routing_policy(&RoutingPolicy {
                policy_id: Uuid::new_v4(),
                service_name: "web".to_string(),
                routing_strategy: RoutingStrategy::RoundRobin,
                timeout_ms: 3000,
                retries: 2,
            })
            .await
            .unwrap();

        router
            .add_weighted_destination(&WeightedDestination { destination_id: Uuid::new_v4(), service_name: "web".to_string(), version: "v1".to_string(), weight: 50 })
            .await
            .unwrap();
        router
            .add_weighted_destination(&WeightedDestination { destination_id: Uuid::new_v4(), service_name: "web".to_string(), version: "v2".to_string(), weight: 50 })
            .await
            .unwrap();

        let r1 = router.route_request("web").await.unwrap();
        let r2 = router.route_request("web").await.unwrap();
        let r3 = router.route_request("web").await.unwrap();

        // Round robin over 2 destinations must alternate, not repeat the
        // same destination or return a static strategy-name string.
        assert_ne!(r1, r2);
        assert_eq!(r1, r3);
        assert!(r1.starts_with("web:"));
    }

    #[tokio::test]
    async fn test_route_request_no_destinations_fails() {
        let router = TrafficRouter::new();
        router
            .register_routing_policy(&RoutingPolicy { policy_id: Uuid::new_v4(), service_name: "empty".to_string(), routing_strategy: RoutingStrategy::Random, timeout_ms: 1000, retries: 0 })
            .await
            .unwrap();

        let result = router.route_request("empty").await;
        assert!(matches!(result, Err(TrafficError::NoDestinationsAvailable)));
    }

    #[tokio::test]
    async fn test_route_request_unknown_policy_fails() {
        let router = TrafficRouter::new();
        let result = router.route_request("nope").await;
        assert!(matches!(result, Err(TrafficError::PolicyNotFound)));
    }

    #[tokio::test]
    async fn test_route_request_least_connections_prefers_idle_destination() {
        let router = TrafficRouter::new();
        router
            .register_routing_policy(&RoutingPolicy { policy_id: Uuid::new_v4(), service_name: "svc".to_string(), routing_strategy: RoutingStrategy::LeastConnections, timeout_ms: 1000, retries: 0 })
            .await
            .unwrap();

        let busy_id = Uuid::new_v4();
        let idle_id = Uuid::new_v4();
        router.add_weighted_destination(&WeightedDestination { destination_id: busy_id, service_name: "svc".to_string(), version: "busy".to_string(), weight: 50 }).await.unwrap();
        router.add_weighted_destination(&WeightedDestination { destination_id: idle_id, service_name: "svc".to_string(), version: "idle".to_string(), weight: 50 }).await.unwrap();

        // Drive up the "busy" destination's connection count directly.
        for _ in 0..5 {
            router.connection_counts.entry(busy_id).or_insert_with(|| AtomicU64::new(0)).fetch_add(1, Ordering::SeqCst);
        }

        let chosen = router.route_request("svc").await.unwrap();
        assert_eq!(chosen, "svc:idle");
    }

    #[tokio::test]
    async fn test_add_weighted_destination() {
        let router = TrafficRouter::new();
        let dest = WeightedDestination {
            destination_id: Uuid::new_v4(),
            service_name: "api".to_string(),
            version: "v1".to_string(),
            weight: 80,
        };

        router.add_weighted_destination(&dest).await.unwrap();
    }

    #[tokio::test]
    async fn test_add_weighted_destination_rejects_invalid_weight() {
        let router = TrafficRouter::new();
        let dest = WeightedDestination { destination_id: Uuid::new_v4(), service_name: "api".to_string(), version: "v1".to_string(), weight: 150 };
        let result = router.add_weighted_destination(&dest).await;
        assert!(matches!(result, Err(TrafficError::InvalidWeight)));
    }

    #[tokio::test]
    async fn test_start_canary_deployment() {
        let router = TrafficRouter::new();
        let deployment = CanaryDeployment {
            deployment_id: Uuid::new_v4(),
            service_name: "api".to_string(),
            stable_version: "v1.0".to_string(),
            canary_version: "v1.1".to_string(),
            canary_traffic_percent: 10,
            status: DeploymentStatus::InProgress,
        };

        router.start_canary_deployment(&deployment).await.unwrap();
    }
}
