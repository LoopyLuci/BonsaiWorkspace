use crate::{RoutingPolicy, RoutingStrategy, WeightedDestination, TrafficShapingPolicy, TrafficError, TrafficResult, CanaryDeployment, DeploymentStatus};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct TrafficRouter {
    policies: Arc<DashMap<String, RoutingPolicy>>,
    weighted_destinations: Arc<DashMap<Uuid, WeightedDestination>>,
    shaping_policies: Arc<DashMap<String, TrafficShapingPolicy>>,
    canary_deployments: Arc<DashMap<Uuid, CanaryDeployment>>,
}

impl TrafficRouter {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(DashMap::new()),
            weighted_destinations: Arc::new(DashMap::new()),
            shaping_policies: Arc::new(DashMap::new()),
            canary_deployments: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_routing_policy(&self, policy: &RoutingPolicy) -> TrafficResult<()> {
        self.policies.insert(policy.service_name.clone(), policy.clone());
        Ok(())
    }

    pub async fn route_request(&self, service_name: &str) -> TrafficResult<String> {
        if let Some(policy) = self.policies.get(service_name) {
            match policy.routing_strategy {
                RoutingStrategy::RoundRobin => Ok("round-robin".to_string()),
                RoutingStrategy::LeastConnections => Ok("least-connections".to_string()),
                RoutingStrategy::WeightedDistribution => Ok("weighted".to_string()),
                RoutingStrategy::Random => Ok("random".to_string()),
            }
        } else {
            Err(TrafficError::PolicyNotFound)
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
    async fn test_route_request() {
        let router = TrafficRouter::new();
        let policy = RoutingPolicy {
            policy_id: Uuid::new_v4(),
            service_name: "web".to_string(),
            routing_strategy: RoutingStrategy::WeightedDistribution,
            timeout_ms: 3000,
            retries: 2,
        };

        router.register_routing_policy(&policy).await.unwrap();
        let result = router.route_request("web").await.unwrap();
        assert_eq!(result, "weighted");
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
