use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct VirtualHostId(pub Uuid);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DomainName(pub String);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VirtualHost {
    pub id: VirtualHostId,
    pub domain: DomainName,
    pub aliases: Vec<DomainName>,
    pub backend_urls: Vec<String>,
    pub tls_enabled: bool,
    pub certificate_id: Option<String>,
    pub root_path: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: HashMap<String, String>,
}

impl VirtualHost {
    pub fn new(domain: DomainName, root_path: String) -> Self {
        Self {
            id: VirtualHostId(Uuid::new_v4()),
            domain,
            aliases: Vec::new(),
            backend_urls: Vec::new(),
            tls_enabled: false,
            certificate_id: None,
            root_path,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: HashMap::new(),
        }
    }

    pub fn with_tls(mut self, cert_id: String) -> Self {
        self.tls_enabled = true;
        self.certificate_id = Some(cert_id);
        self
    }

    pub fn add_backend(mut self, url: String) -> Self {
        self.backend_urls.push(url);
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TlsCertificate {
    pub id: String,
    pub domain: DomainName,
    pub certificate_pem: String,
    pub private_key_pem: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub issuer: String,
    pub self_signed: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum HttpVersion {
    Http10,
    Http11,
    Http2,
    Http3,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub http_version: HttpVersion,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status_code: u16,
    pub http_version: HttpVersion,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReverseProxyConfig {
    pub backend_urls: Vec<String>,
    pub load_balance_policy: LoadBalancePolicy,
    pub connection_timeout_secs: u64,
    pub read_timeout_secs: u64,
    pub write_timeout_secs: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LoadBalancePolicy {
    RoundRobin,
    LeastConnections,
    Random,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityHeaders {
    pub strict_transport_security: bool,
    pub content_security_policy: Option<String>,
    pub x_frame_options: Option<String>,
    pub x_content_type_options: Option<String>,
    pub x_xss_protection: Option<String>,
}

impl Default for SecurityHeaders {
    fn default() -> Self {
        Self {
            strict_transport_security: true,
            content_security_policy: Some("default-src 'self'".to_string()),
            x_frame_options: Some("DENY".to_string()),
            x_content_type_options: Some("nosniff".to_string()),
            x_xss_protection: Some("1; mode=block".to_string()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub enabled: bool,
    pub min_size_bytes: usize,
    pub algorithms: Vec<CompressionAlgorithm>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    Gzip,
    Brotli,
    Deflate,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_size_bytes: 1024,
            algorithms: vec![CompressionAlgorithm::Brotli, CompressionAlgorithm::Gzip],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachingConfig {
    pub enabled: bool,
    pub default_ttl_secs: u32,
    pub max_size_bytes: usize,
}

impl Default for CachingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_ttl_secs: 3600,
            max_size_bytes: 100 * 1024 * 1024,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_host_creation() {
        let domain = DomainName("example.com".to_string());
        let vhost = VirtualHost::new(domain.clone(), "/var/www".to_string());
        assert_eq!(vhost.domain, domain);
        assert!(!vhost.tls_enabled);
    }

    #[test]
    fn test_virtual_host_with_tls() {
        let domain = DomainName("example.com".to_string());
        let vhost = VirtualHost::new(domain, "/var/www".to_string())
            .with_tls("cert-123".to_string());
        assert!(vhost.tls_enabled);
        assert_eq!(vhost.certificate_id, Some("cert-123".to_string()));
    }

    #[test]
    fn test_security_headers_defaults() {
        let headers = SecurityHeaders::default();
        assert!(headers.strict_transport_security);
        assert!(headers.content_security_policy.is_some());
    }

    #[test]
    fn test_compression_config_defaults() {
        let config = CompressionConfig::default();
        assert!(config.enabled);
        assert_eq!(config.algorithms.len(), 2);
    }

    #[test]
    fn test_domain_name_equality() {
        let domain1 = DomainName("example.com".to_string());
        let domain2 = DomainName("example.com".to_string());
        assert_eq!(domain1, domain2);
    }
}
