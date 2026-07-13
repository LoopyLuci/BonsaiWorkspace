use crate::{IntegrationConfig, IntegrationError, IntegrationResult, ServiceEndpoint, ServiceId, ServiceType};
use dashmap::DashMap;
use std::sync::Arc;

pub struct ServiceIntegration {
    #[allow(dead_code)]
    config: Arc<IntegrationConfig>,
    endpoints: Arc<DashMap<String, ServiceEndpoint>>,
}

impl ServiceIntegration {
    pub fn new(config: IntegrationConfig) -> Self {
        Self {
            config: Arc::new(config),
            endpoints: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_endpoint(&self, endpoint: ServiceEndpoint) -> IntegrationResult<()> {
        self.endpoints
            .insert(endpoint.service_id.0.clone(), endpoint);
        Ok(())
    }

    pub async fn get_endpoint(&self, service_id: &ServiceId) -> IntegrationResult<ServiceEndpoint> {
        self.endpoints
            .get(&service_id.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| IntegrationError::ServiceUnavailable(service_id.0.clone()))
    }

    pub async fn list_endpoints(&self, service_type: ServiceType) -> IntegrationResult<Vec<ServiceEndpoint>> {
        let endpoints: Vec<ServiceEndpoint> = self
            .endpoints
            .iter()
            .filter(|entry| entry.value().service_type == service_type)
            .map(|entry| entry.value().clone())
            .collect();

        Ok(endpoints)
    }

    pub async fn deregister_endpoint(&self, service_id: &ServiceId) -> IntegrationResult<()> {
        self.endpoints.remove(&service_id.0);
        Ok(())
    }

    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }

    pub async fn resolve_service(&self, service_type: ServiceType) -> IntegrationResult<ServiceEndpoint> {
        let endpoints = self.list_endpoints(service_type).await?;

        if endpoints.is_empty() {
            return Err(IntegrationError::ServiceUnavailable(
                service_type.to_string().to_string(),
            ));
        }

        Ok(endpoints[0].clone())
    }

    pub async fn validate_service_endpoint(&self, endpoint: &ServiceEndpoint) -> IntegrationResult<()> {
        if endpoint.port == 0 {
            return Err(IntegrationError::ConfigurationError(
                "Invalid port number".to_string(),
            ));
        }

        if endpoint.host.is_empty() {
            return Err(IntegrationError::ConfigurationError(
                "Host cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    pub async fn get_service_url(&self, service_id: &ServiceId) -> IntegrationResult<String> {
        let endpoint = self.get_endpoint(service_id).await?;
        let protocol = if endpoint.tls_enabled { "https" } else { "http" };
        Ok(format!("{}://{}:{}", protocol, endpoint.host, endpoint.port))
    }
}

impl Default for ServiceIntegration {
    fn default() -> Self {
        Self::new(IntegrationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_endpoint() {
        let integration = ServiceIntegration::default();
        let endpoint = ServiceEndpoint {
            service_id: ServiceId("web1".to_string()),
            service_type: ServiceType::WebHosting,
            host: "localhost".to_string(),
            port: 443,
            tls_enabled: true,
        };

        integration.register_endpoint(endpoint).await.unwrap();
        assert_eq!(integration.endpoint_count(), 1);
    }

    #[tokio::test]
    async fn test_get_endpoint() {
        let integration = ServiceIntegration::default();
        let endpoint = ServiceEndpoint {
            service_id: ServiceId("web1".to_string()),
            service_type: ServiceType::WebHosting,
            host: "localhost".to_string(),
            port: 443,
            tls_enabled: true,
        };

        integration.register_endpoint(endpoint.clone()).await.unwrap();

        let retrieved = integration.get_endpoint(&endpoint.service_id).await.unwrap();
        assert_eq!(retrieved.service_id, endpoint.service_id);
    }

    #[tokio::test]
    async fn test_list_endpoints_by_type() {
        let integration = ServiceIntegration::default();

        let web_endpoint = ServiceEndpoint {
            service_id: ServiceId("web1".to_string()),
            service_type: ServiceType::WebHosting,
            host: "localhost".to_string(),
            port: 443,
            tls_enabled: true,
        };

        let dns_endpoint = ServiceEndpoint {
            service_id: ServiceId("dns1".to_string()),
            service_type: ServiceType::DnsRouter,
            host: "localhost".to_string(),
            port: 53,
            tls_enabled: false,
        };

        integration.register_endpoint(web_endpoint).await.unwrap();
        integration.register_endpoint(dns_endpoint).await.unwrap();

        let web_endpoints = integration.list_endpoints(ServiceType::WebHosting).await.unwrap();
        assert_eq!(web_endpoints.len(), 1);
    }

    #[tokio::test]
    async fn test_deregister_endpoint() {
        let integration = ServiceIntegration::default();
        let endpoint = ServiceEndpoint {
            service_id: ServiceId("web1".to_string()),
            service_type: ServiceType::WebHosting,
            host: "localhost".to_string(),
            port: 443,
            tls_enabled: true,
        };

        integration.register_endpoint(endpoint.clone()).await.unwrap();
        assert_eq!(integration.endpoint_count(), 1);

        integration.deregister_endpoint(&endpoint.service_id).await.unwrap();
        assert_eq!(integration.endpoint_count(), 0);
    }

    #[tokio::test]
    async fn test_resolve_service() {
        let integration = ServiceIntegration::default();
        let endpoint = ServiceEndpoint {
            service_id: ServiceId("web1".to_string()),
            service_type: ServiceType::WebHosting,
            host: "localhost".to_string(),
            port: 443,
            tls_enabled: true,
        };

        integration.register_endpoint(endpoint).await.unwrap();

        let resolved = integration.resolve_service(ServiceType::WebHosting).await.unwrap();
        assert_eq!(resolved.service_type, ServiceType::WebHosting);
    }

    #[tokio::test]
    async fn test_get_service_url() {
        let integration = ServiceIntegration::default();
        let endpoint = ServiceEndpoint {
            service_id: ServiceId("web1".to_string()),
            service_type: ServiceType::WebHosting,
            host: "example.com".to_string(),
            port: 443,
            tls_enabled: true,
        };

        integration.register_endpoint(endpoint.clone()).await.unwrap();

        let url = integration.get_service_url(&endpoint.service_id).await.unwrap();
        assert_eq!(url, "https://example.com:443");
    }

    #[tokio::test]
    async fn test_validate_endpoint() {
        let integration = ServiceIntegration::default();

        let valid_endpoint = ServiceEndpoint {
            service_id: ServiceId("web1".to_string()),
            service_type: ServiceType::WebHosting,
            host: "localhost".to_string(),
            port: 443,
            tls_enabled: true,
        };

        assert!(integration.validate_service_endpoint(&valid_endpoint).await.is_ok());

        let invalid_endpoint = ServiceEndpoint {
            service_id: ServiceId("web2".to_string()),
            service_type: ServiceType::WebHosting,
            host: "".to_string(),
            port: 443,
            tls_enabled: true,
        };

        assert!(integration.validate_service_endpoint(&invalid_endpoint).await.is_err());
    }
}
