use crate::{NetworkPolicy, MtlsPolicy, Certificate, NetworkSegment, AccessControl, PolicyError, PolicyResult, Action, IsolationLevel};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct PolicyManager {
    policies: Arc<DashMap<Uuid, NetworkPolicy>>,
    mtls_policies: Arc<DashMap<Uuid, MtlsPolicy>>,
    certificates: Arc<DashMap<Uuid, Certificate>>,
    segments: Arc<DashMap<Uuid, NetworkSegment>>,
    access_controls: Arc<DashMap<Uuid, AccessControl>>,
}

impl PolicyManager {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(DashMap::new()),
            mtls_policies: Arc::new(DashMap::new()),
            certificates: Arc::new(DashMap::new()),
            segments: Arc::new(DashMap::new()),
            access_controls: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_network_policy(&self, policy: &NetworkPolicy) -> PolicyResult<()> {
        self.policies.insert(policy.policy_id, policy.clone());
        Ok(())
    }

    pub async fn check_access(&self, source: &str, destination: &str, port: u16) -> PolicyResult<bool> {
        for entry in self.policies.iter() {
            let policy = entry.value();
            if policy.source == source && policy.destination == destination && policy.port == port {
                return Ok(policy.action == Action::Allow);
            }
        }

        Err(PolicyError::PolicyNotFound)
    }

    pub async fn enable_mtls(&self, mtls: &MtlsPolicy) -> PolicyResult<()> {
        self.mtls_policies.insert(mtls.policy_id, mtls.clone());
        Ok(())
    }

    pub async fn get_mtls_policy(&self, policy_id: Uuid) -> PolicyResult<MtlsPolicy> {
        self.mtls_policies
            .get(&policy_id)
            .map(|p| p.clone())
            .ok_or(PolicyError::MtlsConfigurationFailed)
    }

    pub async fn register_certificate(&self, cert: &Certificate) -> PolicyResult<()> {
        self.certificates.insert(cert.cert_id, cert.clone());
        Ok(())
    }

    /// Look up a registered certificate and confirm it's actually marked
    /// valid, rather than certificates being write-only (registered but
    /// never checkable/consulted by anything).
    pub async fn verify_certificate(&self, cert_id: Uuid) -> PolicyResult<Certificate> {
        let cert = self.certificates.get(&cert_id).ok_or(PolicyError::CertificateInvalid)?;
        if !cert.is_valid {
            return Err(PolicyError::CertificateInvalid);
        }
        Ok(cert.clone())
    }

    pub async fn create_network_segment(&self, segment: &NetworkSegment) -> PolicyResult<()> {
        self.segments.insert(segment.segment_id, segment.clone());
        Ok(())
    }

    pub async fn get_network_segment(&self, segment_id: Uuid) -> PolicyResult<NetworkSegment> {
        self.segments.get(&segment_id).map(|s| s.clone()).ok_or(PolicyError::SegmentNotFound)
    }

    /// Determine whether traffic may cross between two network segments
    /// based on their isolation levels: `Strict` segments never permit
    /// cross-segment traffic (not even to another Strict segment) except
    /// to themselves, while lower isolation levels are more permissive.
    pub async fn segments_can_communicate(&self, segment_a_id: Uuid, segment_b_id: Uuid) -> PolicyResult<bool> {
        let a = self.get_network_segment(segment_a_id).await?;
        let b = self.get_network_segment(segment_b_id).await?;

        if a.segment_id == b.segment_id {
            return Ok(true);
        }

        let strictest = a.isolation_level.max(b.isolation_level);
        Ok(strictest != IsolationLevel::Strict)
    }

    pub async fn set_access_control(&self, control: &AccessControl) -> PolicyResult<()> {
        self.access_controls.insert(control.control_id, control.clone());
        Ok(())
    }

    pub async fn check_inter_service_access(&self, service_a: &str, service_b: &str) -> PolicyResult<bool> {
        for entry in self.access_controls.iter() {
            let control = entry.value();
            if control.service_a == service_a && control.service_b == service_b {
                return Ok(control.allowed);
            }
        }

        Err(PolicyError::AccessDenied)
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }
}

impl Default for PolicyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_network_policy() {
        let manager = PolicyManager::new();
        let policy = NetworkPolicy {
            policy_id: Uuid::new_v4(),
            name: "allow-api".to_string(),
            source: "frontend".to_string(),
            destination: "api".to_string(),
            port: 8080,
            protocol: "TCP".to_string(),
            action: Action::Allow,
        };

        manager.create_network_policy(&policy).await.unwrap();
        assert_eq!(manager.policy_count(), 1);
    }

    #[tokio::test]
    async fn test_check_access() {
        let manager = PolicyManager::new();
        let policy = NetworkPolicy {
            policy_id: Uuid::new_v4(),
            name: "allow".to_string(),
            source: "svc1".to_string(),
            destination: "svc2".to_string(),
            port: 443,
            protocol: "TCP".to_string(),
            action: Action::Allow,
        };

        manager.create_network_policy(&policy).await.unwrap();
        let allowed = manager.check_access("svc1", "svc2", 443).await.unwrap();
        assert!(allowed);
    }

    #[tokio::test]
    async fn test_enable_mtls() {
        let manager = PolicyManager::new();
        let mtls = MtlsPolicy {
            policy_id: Uuid::new_v4(),
            name: "strict-mtls".to_string(),
            enabled: true,
            min_tls_version: "1.3".to_string(),
            cipher_suites: vec!["TLS_AES_256_GCM_SHA384".to_string()],
        };

        manager.enable_mtls(&mtls).await.unwrap();
    }

    #[tokio::test]
    async fn test_set_access_control() {
        let manager = PolicyManager::new();
        let control = AccessControl {
            control_id: Uuid::new_v4(),
            service_a: "frontend".to_string(),
            service_b: "backend".to_string(),
            allowed: true,
            reason: Some("Production traffic".to_string()),
        };

        manager.set_access_control(&control).await.unwrap();
        let allowed = manager.check_inter_service_access("frontend", "backend").await.unwrap();
        assert!(allowed);
    }

    #[tokio::test]
    async fn test_verify_certificate_valid_and_invalid() {
        let manager = PolicyManager::new();
        let valid_cert = Certificate {
            cert_id: Uuid::new_v4(),
            common_name: "api.example.com".to_string(),
            issuer: "Internal CA".to_string(),
            valid_from: "2026-01-01".to_string(),
            valid_to: "2027-01-01".to_string(),
            is_valid: true,
        };
        let expired_cert = Certificate {
            cert_id: Uuid::new_v4(),
            common_name: "old.example.com".to_string(),
            issuer: "Internal CA".to_string(),
            valid_from: "2020-01-01".to_string(),
            valid_to: "2021-01-01".to_string(),
            is_valid: false,
        };

        manager.register_certificate(&valid_cert).await.unwrap();
        manager.register_certificate(&expired_cert).await.unwrap();

        assert!(manager.verify_certificate(valid_cert.cert_id).await.is_ok());
        assert!(matches!(
            manager.verify_certificate(expired_cert.cert_id).await,
            Err(PolicyError::CertificateInvalid)
        ));
        assert!(matches!(
            manager.verify_certificate(Uuid::new_v4()).await,
            Err(PolicyError::CertificateInvalid)
        ));
    }

    #[tokio::test]
    async fn test_segments_can_communicate_respects_strict_isolation() {
        let manager = PolicyManager::new();
        let strict_segment = NetworkSegment {
            segment_id: Uuid::new_v4(),
            name: "payments".to_string(),
            cidr: "10.0.1.0/24".to_string(),
            isolation_level: IsolationLevel::Strict,
        };
        let low_segment = NetworkSegment {
            segment_id: Uuid::new_v4(),
            name: "public".to_string(),
            cidr: "10.0.2.0/24".to_string(),
            isolation_level: IsolationLevel::Low,
        };

        manager.create_network_segment(&strict_segment).await.unwrap();
        manager.create_network_segment(&low_segment).await.unwrap();

        // A Strict segment must not be able to talk to any other segment.
        assert!(!manager
            .segments_can_communicate(strict_segment.segment_id, low_segment.segment_id)
            .await
            .unwrap());
        // But it can always talk to itself.
        assert!(manager
            .segments_can_communicate(strict_segment.segment_id, strict_segment.segment_id)
            .await
            .unwrap());
        // Two non-strict segments may communicate.
        let another_low = NetworkSegment {
            segment_id: Uuid::new_v4(),
            name: "dmz".to_string(),
            cidr: "10.0.3.0/24".to_string(),
            isolation_level: IsolationLevel::Medium,
        };
        manager.create_network_segment(&another_low).await.unwrap();
        assert!(manager
            .segments_can_communicate(low_segment.segment_id, another_low.segment_id)
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn test_check_access_unknown_policy_fails() {
        let manager = PolicyManager::new();
        let result = manager.check_access("unknown_a", "unknown_b", 1).await;
        assert!(matches!(result, Err(PolicyError::PolicyNotFound)));
    }
}
