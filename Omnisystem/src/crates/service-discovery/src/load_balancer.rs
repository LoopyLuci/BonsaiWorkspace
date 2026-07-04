use crate::{DiscoveryError, DiscoveryResult, LoadBalancingPolicy, ServiceInstance};
use dashmap::DashMap;
use std::sync::Arc;

pub struct LoadBalancer {
    policies: Arc<DashMap<String, LoadBalancingPolicy>>,
    counters: Arc<DashMap<String, u64>>,
}

impl LoadBalancer {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(DashMap::new()),
            counters: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_policy(&self, service_name: &str, policy: &LoadBalancingPolicy) -> DiscoveryResult<()> {
        self.policies.insert(service_name.to_string(), policy.clone());
        self.counters.insert(service_name.to_string(), 0);
        Ok(())
    }

    pub async fn select_instance(
        &self,
        service_name: &str,
        instances: &[ServiceInstance],
    ) -> DiscoveryResult<ServiceInstance> {
        if instances.is_empty() {
            return Err(DiscoveryError::LoadBalancingFailed);
        }

        if let Some(policy) = self.policies.get(service_name) {
            if policy.policy_type == "round_robin" {
                let mut counter = self.counters.entry(service_name.to_string()).or_insert(0);
                let idx = (*counter as usize) % instances.len();
                *counter += 1;
                Ok(instances[idx].clone())
            } else if policy.policy_type == "random" {
                let idx = (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as usize) % instances.len();
                Ok(instances[idx].clone())
            } else {
                Ok(instances[0].clone())
            }
        } else {
            Ok(instances[0].clone())
        }
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }
}

impl Default for LoadBalancer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use chrono::Utc;
    use crate::ServiceStatus;

    #[tokio::test]
    async fn test_register_policy() {
        let lb = LoadBalancer::new();
        let policy = LoadBalancingPolicy {
            policy_type: "round_robin".to_string(),
            weight_map: HashMap::new(),
        };

        lb.register_policy("api", &policy).await.unwrap();
        assert_eq!(lb.policy_count(), 1);
    }

    #[tokio::test]
    async fn test_round_robin_selection() {
        let lb = LoadBalancer::new();
        let policy = LoadBalancingPolicy {
            policy_type: "round_robin".to_string(),
            weight_map: HashMap::new(),
        };

        lb.register_policy("api", &policy).await.unwrap();

        let instances = vec![
            ServiceInstance {
                instance_id: "i1".to_string(),
                service_name: "api".to_string(),
                host: "host1".to_string(),
                port: 8080,
                status: ServiceStatus::Healthy,
                registered_at: Utc::now(),
                last_heartbeat: Utc::now(),
                metadata: HashMap::new(),
            },
            ServiceInstance {
                instance_id: "i2".to_string(),
                service_name: "api".to_string(),
                host: "host2".to_string(),
                port: 8080,
                status: ServiceStatus::Healthy,
                registered_at: Utc::now(),
                last_heartbeat: Utc::now(),
                metadata: HashMap::new(),
            },
        ];

        let selected1 = lb.select_instance("api", &instances).await.unwrap();
        let selected2 = lb.select_instance("api", &instances).await.unwrap();

        assert_ne!(selected1.instance_id, selected2.instance_id);
    }

    #[tokio::test]
    async fn test_random_selection() {
        let lb = LoadBalancer::new();
        let policy = LoadBalancingPolicy {
            policy_type: "random".to_string(),
            weight_map: HashMap::new(),
        };

        lb.register_policy("api", &policy).await.unwrap();

        let instances = vec![
            ServiceInstance {
                instance_id: "i1".to_string(),
                service_name: "api".to_string(),
                host: "host1".to_string(),
                port: 8080,
                status: ServiceStatus::Healthy,
                registered_at: Utc::now(),
                last_heartbeat: Utc::now(),
                metadata: HashMap::new(),
            },
        ];

        let selected = lb.select_instance("api", &instances).await.unwrap();
        assert_eq!(selected.instance_id, "i1");
    }

    #[tokio::test]
    async fn test_empty_instances() {
        let lb = LoadBalancer::new();
        let instances: Vec<ServiceInstance> = vec![];

        let result = lb.select_instance("api", &instances).await;
        assert!(result.is_err());
    }
}
