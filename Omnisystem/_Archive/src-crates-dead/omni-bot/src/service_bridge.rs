use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct ServiceBridge {
    iot_service: Arc<std::sync::Mutex<Option<String>>>,
    search_service: Arc<std::sync::Mutex<Option<String>>>,
    fabrication_service: Arc<std::sync::Mutex<Option<String>>>,
    agent_service: Arc<std::sync::Mutex<Option<String>>>,
    network_service: Arc<std::sync::Mutex<Option<String>>>,
    metrics: Arc<DashMap<String, u64>>,
}

impl ServiceBridge {
    pub fn new() -> Self {
        Self {
            iot_service: Arc::new(std::sync::Mutex::new(None)),
            search_service: Arc::new(std::sync::Mutex::new(None)),
            fabrication_service: Arc::new(std::sync::Mutex::new(None)),
            agent_service: Arc::new(std::sync::Mutex::new(None)),
            network_service: Arc::new(std::sync::Mutex::new(None)),
            metrics: Arc::new(DashMap::new()),
        }
    }

    pub async fn connect_all(&self) -> Result<()> {
        *self.iot_service.lock().unwrap() = Some("IoT connected".to_string());
        *self.search_service.lock().unwrap() = Some("Search connected".to_string());
        *self.fabrication_service.lock().unwrap() = Some("Fabrication connected".to_string());
        *self.agent_service.lock().unwrap() = Some("Agents connected".to_string());
        *self.network_service.lock().unwrap() = Some("Network connected".to_string());
        tracing::info!("All services connected");
        Ok(())
    }

    pub async fn route_to_iot(&self, payload: &str) -> Result<String> {
        self.metrics.entry("iot_requests".to_string()).or_insert(0).add_assign(1);
        tracing::info!("Routing to IoT service");
        Ok(format!("IoT processed: {}", payload))
    }

    pub async fn route_to_search(&self, payload: &str) -> Result<String> {
        self.metrics.entry("search_requests".to_string()).or_insert(0).add_assign(1);
        tracing::info!("Routing to Search service");
        Ok(format!("Search processed: {}", payload))
    }

    pub async fn route_to_fabrication(&self, payload: &str) -> Result<String> {
        self.metrics.entry("fabrication_requests".to_string()).or_insert(0).add_assign(1);
        tracing::info!("Routing to Fabrication service");
        Ok(format!("Fabrication processed: {}", payload))
    }

    pub async fn route_to_agents(&self, payload: &str) -> Result<String> {
        self.metrics.entry("agent_requests".to_string()).or_insert(0).add_assign(1);
        tracing::info!("Routing to Agent service");
        Ok(format!("Agents processed: {}", payload))
    }

    pub async fn route_to_network(&self, payload: &str) -> Result<String> {
        self.metrics.entry("network_requests".to_string()).or_insert(0).add_assign(1);
        tracing::info!("Routing to Network service");
        Ok(format!("Network processed: {}", payload))
    }

    pub fn get_request_count(&self, service: &str) -> u64 {
        self.metrics
            .get(&format!("{}_requests", service))
            .map(|ref_| *ref_.value())
            .unwrap_or(0)
    }
}

impl Default for ServiceBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_bridge() {
        let bridge = ServiceBridge::new();
        assert!(bridge.connect_all().await.is_ok());
    }

    #[tokio::test]
    async fn test_routing() {
        let bridge = ServiceBridge::new();
        bridge.connect_all().await.unwrap();
        
        let iot_result = bridge.route_to_iot("test").await.unwrap();
        assert!(iot_result.contains("IoT processed"));
        assert_eq!(bridge.get_request_count("iot"), 1);
    }
}
