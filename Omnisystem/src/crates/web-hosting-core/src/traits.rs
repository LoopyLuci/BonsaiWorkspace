use crate::{
    DomainName, HttpRequest, HttpResponse, TlsCertificate, VirtualHost, VirtualHostId,
    WebResult,
};
use async_trait::async_trait;

#[async_trait]
pub trait WebHostingManager: Send + Sync {
    async fn create_virtual_host(&self, vhost: VirtualHost) -> WebResult<VirtualHost>;

    async fn delete_virtual_host(&self, id: &VirtualHostId) -> WebResult<()>;

    async fn get_virtual_host(&self, id: &VirtualHostId) -> WebResult<VirtualHost>;

    async fn get_virtual_host_by_domain(&self, domain: &DomainName) -> WebResult<VirtualHost>;

    async fn list_virtual_hosts(&self) -> WebResult<Vec<VirtualHost>>;

    async fn update_virtual_host(&self, id: &VirtualHostId, vhost: VirtualHost)
        -> WebResult<()>;

    async fn add_domain_alias(
        &self,
        id: &VirtualHostId,
        alias: DomainName,
    ) -> WebResult<()>;

    async fn remove_domain_alias(&self, id: &VirtualHostId, alias: &DomainName)
        -> WebResult<()>;
}

#[async_trait]
pub trait ReverseProxyHandler: Send + Sync {
    async fn handle_request(
        &self,
        vhost: &VirtualHost,
        request: HttpRequest,
    ) -> WebResult<HttpResponse>;

    async fn health_check_backend(&self, backend_url: &str) -> WebResult<bool>;

    async fn get_backend_status(&self, vhost_id: &VirtualHostId) -> WebResult<Vec<bool>>;
}

#[async_trait]
pub trait SecurityManager: Send + Sync {
    async fn validate_host_header(&self, host: &str) -> WebResult<bool>;

    async fn validate_request_headers(&self, request: &HttpRequest) -> WebResult<()>;

    async fn apply_security_headers(&self, response: &mut HttpResponse) -> WebResult<()>;

    async fn rate_limit_check(&self, ip_address: &str) -> WebResult<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_result_ok() {
        let result: WebResult<String> = Ok("success".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_web_result_err() {
        let result: WebResult<String> =
            Err(crate::WebError::VirtualHostNotFound("test.com".to_string()));
        assert!(result.is_err());
    }
}
