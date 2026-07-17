//! CLI demo: register a service endpoint and look it up through the integration layer.

use web_hosting_integration::{IntegrationConfig, ServiceEndpoint, ServiceId, ServiceIntegration, ServiceType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let integration = ServiceIntegration::new(IntegrationConfig::default());

    integration
        .register_endpoint(ServiceEndpoint {
            service_id: ServiceId("web-hosting-1".to_string()),
            service_type: ServiceType::WebHosting,
            host: "10.0.0.10".to_string(),
            port: 8080,
            tls_enabled: true,
        })
        .await?;

    let endpoint = integration.get_endpoint(&ServiceId("web-hosting-1".to_string())).await?;
    println!("Endpoint: {}:{} ({})", endpoint.host, endpoint.port, endpoint.service_type.to_string());

    Ok(())
}
