use crate::AggregatorResult;
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ServiceDependency {
    pub source_service: String,
    pub target_service: String,
    pub call_count: u64,
    pub error_count: u64,
    pub avg_latency_ms: f64,
}

#[derive(Clone, Debug)]
pub struct DistributedServiceGraph {
    pub services: Vec<String>,
    pub dependencies: Vec<ServiceDependency>,
    pub critical_path: Vec<String>,
}

pub struct DistributedSystemsObservability {
    dependencies: Arc<DashMap<String, Vec<ServiceDependency>>>,
    service_graph: Arc<parking_lot::Mutex<DistributedServiceGraph>>,
}

impl DistributedSystemsObservability {
    pub fn new() -> Self {
        Self {
            dependencies: Arc::new(DashMap::new()),
            service_graph: Arc::new(parking_lot::Mutex::new(DistributedServiceGraph {
                services: Vec::new(),
                dependencies: Vec::new(),
                critical_path: Vec::new(),
            })),
        }
    }

    pub async fn record_service_call(
        &self,
        source_service: &str,
        target_service: &str,
        latency_ms: f64,
        error: bool,
    ) -> AggregatorResult<()> {
        let key = format!("{}->{}", source_service, target_service);

        let mut deps = self
            .dependencies
            .entry(key)
            .or_insert_with(Vec::new);

        if let Some(dep) = deps.iter_mut().find(|d| d.source_service == source_service && d.target_service == target_service) {
            dep.call_count += 1;
            if error {
                dep.error_count += 1;
            }
            dep.avg_latency_ms = (dep.avg_latency_ms * (dep.call_count - 1) as f64 + latency_ms) / dep.call_count as f64;
        } else {
            deps.push(ServiceDependency {
                source_service: source_service.to_string(),
                target_service: target_service.to_string(),
                call_count: 1,
                error_count: if error { 1 } else { 0 },
                avg_latency_ms: latency_ms,
            });
        }

        Ok(())
    }

    pub async fn get_service_dependencies(
        &self,
        service: &str,
    ) -> AggregatorResult<Vec<ServiceDependency>> {
        let mut result = Vec::new();

        for entry in self.dependencies.iter() {
            for dep in entry.value().iter() {
                if dep.source_service == service {
                    result.push(dep.clone());
                }
            }
        }

        Ok(result)
    }

    pub async fn build_service_graph(&self) -> AggregatorResult<DistributedServiceGraph> {
        let mut services = std::collections::HashSet::new();
        let mut dependencies = Vec::new();

        for entry in self.dependencies.iter() {
            for dep in entry.value().iter() {
                services.insert(dep.source_service.clone());
                services.insert(dep.target_service.clone());
                dependencies.push(dep.clone());
            }
        }

        let mut graph = self.service_graph.lock();
        graph.services = services.into_iter().collect();
        graph.dependencies = dependencies;
        graph.critical_path = Self::compute_critical_path(&graph.services, &graph.dependencies);

        Ok(graph.clone())
    }

    fn compute_critical_path(
        services: &[String],
        dependencies: &[ServiceDependency],
    ) -> Vec<String> {
        if services.is_empty() {
            return Vec::new();
        }

        let mut path = vec![services[0].clone()];
        let mut current = services[0].clone();

        for _ in 0..services.len() {
            let next = dependencies
                .iter()
                .filter(|d| d.source_service == current)
                .max_by(|a, b| a.avg_latency_ms.partial_cmp(&b.avg_latency_ms).unwrap())
                .map(|d| d.target_service.clone());

            if let Some(next_service) = next {
                if !path.contains(&next_service) {
                    path.push(next_service.clone());
                    current = next_service;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        path
    }

    pub async fn get_critical_path(&self) -> AggregatorResult<Vec<String>> {
        let graph = self.service_graph.lock();
        Ok(graph.critical_path.clone())
    }

    pub fn dependency_count(&self) -> usize {
        self.dependencies
            .iter()
            .map(|entry| entry.value().len())
            .sum()
    }
}

impl Default for DistributedSystemsObservability {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_service_call() {
        let obs = DistributedSystemsObservability::new();

        obs.record_service_call("api-service", "db-service", 50.0, false)
            .await
            .unwrap();

        assert_eq!(obs.dependency_count(), 1);
    }

    #[tokio::test]
    async fn test_get_service_dependencies() {
        let obs = DistributedSystemsObservability::new();

        obs.record_service_call("api-service", "db-service", 50.0, false)
            .await
            .unwrap();
        obs.record_service_call("api-service", "cache-service", 10.0, false)
            .await
            .unwrap();

        let deps = obs.get_service_dependencies("api-service").await.unwrap();
        assert_eq!(deps.len(), 2);
    }

    #[tokio::test]
    async fn test_build_service_graph() {
        let obs = DistributedSystemsObservability::new();

        obs.record_service_call("api", "db", 100.0, false).await.unwrap();
        obs.record_service_call("api", "cache", 50.0, false).await.unwrap();
        obs.record_service_call("db", "backup", 200.0, false).await.unwrap();

        let graph = obs.build_service_graph().await.unwrap();
        assert!(graph.services.contains(&"api".to_string()));
        assert!(graph.services.contains(&"db".to_string()));
    }

    #[tokio::test]
    async fn test_service_call_with_error() {
        let obs = DistributedSystemsObservability::new();

        obs.record_service_call("api", "db", 100.0, false).await.unwrap();
        obs.record_service_call("api", "db", 150.0, true).await.unwrap();

        let deps = obs.get_service_dependencies("api").await.unwrap();
        let db_dep = deps.iter().find(|d| d.target_service == "db").unwrap();

        assert_eq!(db_dep.call_count, 2);
        assert_eq!(db_dep.error_count, 1);
    }

    #[tokio::test]
    async fn test_critical_path_computation() {
        let obs = DistributedSystemsObservability::new();

        obs.record_service_call("frontend", "api", 10.0, false).await.unwrap();
        obs.record_service_call("api", "db", 100.0, false).await.unwrap();
        obs.record_service_call("api", "cache", 20.0, false).await.unwrap();

        obs.build_service_graph().await.unwrap();
        let path = obs.get_critical_path().await.unwrap();
        assert!(!path.is_empty());
    }
}
