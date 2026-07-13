use crate::{DomainName, TlsCertificate, WebError, WebResult};
use chrono::{Duration, Utc};
use dashmap::DashMap;
use std::sync::Arc;

pub struct CertificateManager {
    certificates: Arc<DashMap<String, TlsCertificate>>,
}

impl CertificateManager {
    pub fn new() -> Self {
        Self {
            certificates: Arc::new(DashMap::new()),
        }
    }

    pub fn certificate_count(&self) -> usize {
        self.certificates.len()
    }

    pub async fn store_certificate(
        &self,
        cert_id: String,
        domain: DomainName,
        certificate_pem: String,
        private_key_pem: String,
        expires_at: chrono::DateTime<Utc>,
    ) -> WebResult<TlsCertificate> {
        let cert = TlsCertificate {
            id: cert_id.clone(),
            domain,
            certificate_pem,
            private_key_pem,
            issued_at: Utc::now(),
            expires_at,
            issuer: "Let's Encrypt".to_string(),
            self_signed: false,
        };

        self.certificates.insert(cert_id, cert.clone());
        Ok(cert)
    }

    pub async fn get_certificate(&self, cert_id: &str) -> WebResult<TlsCertificate> {
        self.certificates
            .get(cert_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| WebError::CertificateNotFound(cert_id.to_string()))
    }

    pub async fn get_certificate_by_domain(
        &self,
        domain: &DomainName,
    ) -> WebResult<TlsCertificate> {
        for entry in self.certificates.iter() {
            if &entry.domain == domain {
                return Ok(entry.clone());
            }
        }
        Err(WebError::CertificateNotFound(domain.0.clone()))
    }

    pub async fn check_certificate_expiry(&self, cert_id: &str) -> WebResult<bool> {
        let cert = self.get_certificate(cert_id).await?;
        let now = Utc::now();
        Ok(cert.expires_at > now)
    }

    pub async fn get_expiring_soon(&self, days: i64) -> WebResult<Vec<TlsCertificate>> {
        let now = Utc::now();
        let threshold = now + Duration::days(days);

        let mut expiring = Vec::new();
        for entry in self.certificates.iter() {
            let cert = entry.value();
            if cert.expires_at <= threshold && cert.expires_at > now {
                expiring.push(cert.clone());
            }
        }

        Ok(expiring)
    }

    pub async fn revoke_certificate(&self, cert_id: &str) -> WebResult<()> {
        if self.certificates.remove(cert_id).is_some() {
            Ok(())
        } else {
            Err(WebError::CertificateNotFound(cert_id.to_string()))
        }
    }

    pub async fn list_certificates(&self) -> WebResult<Vec<TlsCertificate>> {
        Ok(self
            .certificates
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }
}

impl Default for CertificateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_retrieve_certificate() {
        let manager = CertificateManager::new();
        let domain = DomainName("example.com".to_string());
        let expires_at = Utc::now() + Duration::days(365);

        let cert = manager
            .store_certificate(
                "cert-123".to_string(),
                domain.clone(),
                "CERT_PEM".to_string(),
                "KEY_PEM".to_string(),
                expires_at,
            )
            .await
            .unwrap();

        assert_eq!(cert.id, "cert-123");
        assert_eq!(cert.domain, domain);
    }

    #[tokio::test]
    async fn test_get_certificate_by_domain() {
        let manager = CertificateManager::new();
        let domain = DomainName("example.com".to_string());
        let expires_at = Utc::now() + Duration::days(365);

        manager
            .store_certificate(
                "cert-123".to_string(),
                domain.clone(),
                "CERT_PEM".to_string(),
                "KEY_PEM".to_string(),
                expires_at,
            )
            .await
            .unwrap();

        let retrieved = manager.get_certificate_by_domain(&domain).await.unwrap();
        assert_eq!(retrieved.domain, domain);
    }

    #[tokio::test]
    async fn test_check_certificate_expiry() {
        let manager = CertificateManager::new();
        let domain = DomainName("example.com".to_string());
        let expires_at = Utc::now() + Duration::days(30);

        manager
            .store_certificate(
                "cert-123".to_string(),
                domain,
                "CERT_PEM".to_string(),
                "KEY_PEM".to_string(),
                expires_at,
            )
            .await
            .unwrap();

        let is_valid = manager.check_certificate_expiry("cert-123").await.unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_get_expiring_soon() {
        let manager = CertificateManager::new();
        let domain = DomainName("example.com".to_string());
        let expires_at = Utc::now() + Duration::days(5);

        manager
            .store_certificate(
                "cert-123".to_string(),
                domain,
                "CERT_PEM".to_string(),
                "KEY_PEM".to_string(),
                expires_at,
            )
            .await
            .unwrap();

        let expiring = manager.get_expiring_soon(10).await.unwrap();
        assert_eq!(expiring.len(), 1);
    }

    #[tokio::test]
    async fn test_revoke_certificate() {
        let manager = CertificateManager::new();
        let domain = DomainName("example.com".to_string());
        let expires_at = Utc::now() + Duration::days(365);

        manager
            .store_certificate(
                "cert-123".to_string(),
                domain,
                "CERT_PEM".to_string(),
                "KEY_PEM".to_string(),
                expires_at,
            )
            .await
            .unwrap();

        manager.revoke_certificate("cert-123").await.unwrap();
        assert_eq!(manager.certificate_count(), 0);
    }
}
