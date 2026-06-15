use crate::{AuditLog, IntegrationError, IntegrationResult, SecurityPolicy, ServiceId};
use chrono::Utc;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct SecurityManager {
    policy: Arc<SecurityPolicy>,
    audit_logs: Arc<DashMap<String, AuditLog>>,
    rate_limiters: Arc<DashMap<String, AtomicU64>>,
    blocked_ips: Arc<DashMap<String, i64>>,
}

impl SecurityManager {
    pub fn new(policy: SecurityPolicy) -> Self {
        Self {
            policy: Arc::new(policy),
            audit_logs: Arc::new(DashMap::new()),
            rate_limiters: Arc::new(DashMap::new()),
            blocked_ips: Arc::new(DashMap::new()),
        }
    }

    pub fn audit_log_count(&self) -> usize {
        self.audit_logs.len()
    }

    pub fn blocked_ip_count(&self) -> usize {
        self.blocked_ips.len()
    }

    pub async fn validate_tls(&self, tls_enabled: bool) -> IntegrationResult<()> {
        if self.policy.require_tls && !tls_enabled {
            return Err(IntegrationError::SecurityViolation(
                "TLS is required".to_string(),
            ));
        }
        Ok(())
    }

    pub async fn check_rate_limit(&self, client_id: &str) -> IntegrationResult<()> {
        let counter = self
            .rate_limiters
            .entry(client_id.to_string())
            .or_insert_with(|| AtomicU64::new(0));

        let count = counter.fetch_add(1, Ordering::Relaxed);

        if count >= self.policy.rate_limit_requests_per_sec as u64 {
            return Err(IntegrationError::RateLimitExceeded);
        }

        Ok(())
    }

    pub async fn log_audit(
        &self,
        service_id: &ServiceId,
        action: &str,
        user: &str,
        result: &str,
        details: HashMap<String, String>,
    ) -> IntegrationResult<()> {
        let log = AuditLog {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            service_id: service_id.clone(),
            action: action.to_string(),
            user: user.to_string(),
            result: result.to_string(),
            details,
        };

        self.audit_logs.insert(log.id.clone(), log);
        Ok(())
    }

    pub async fn get_audit_logs(&self, service_id: &ServiceId) -> IntegrationResult<Vec<AuditLog>> {
        let logs: Vec<AuditLog> = self
            .audit_logs
            .iter()
            .filter(|entry| entry.value().service_id == *service_id)
            .map(|entry| entry.value().clone())
            .collect();

        Ok(logs)
    }

    pub async fn block_ip(&self, ip: &str) -> IntegrationResult<()> {
        let until = Utc::now().timestamp() + 3600;
        self.blocked_ips.insert(ip.to_string(), until);
        Ok(())
    }

    pub async fn is_ip_blocked(&self, ip: &str) -> IntegrationResult<bool> {
        if let Some(entry) = self.blocked_ips.get(ip) {
            let now = Utc::now().timestamp();
            Ok(*entry > now)
        } else {
            Ok(false)
        }
    }

    pub async fn unblock_ip(&self, ip: &str) -> IntegrationResult<()> {
        self.blocked_ips.remove(ip);
        Ok(())
    }

    pub async fn reset_rate_limits(&self) -> IntegrationResult<()> {
        self.rate_limiters.clear();
        Ok(())
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new(SecurityPolicy::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validate_tls_required() {
        let policy = SecurityPolicy {
            require_tls: true,
            ..Default::default()
        };
        let manager = SecurityManager::new(policy);

        let result = manager.validate_tls(false).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_tls_allowed() {
        let policy = SecurityPolicy {
            require_tls: false,
            ..Default::default()
        };
        let manager = SecurityManager::new(policy);

        let result = manager.validate_tls(false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let policy = SecurityPolicy {
            rate_limit_requests_per_sec: 5,
            ..Default::default()
        };
        let manager = SecurityManager::new(policy);

        for _ in 0..5 {
            assert!(manager.check_rate_limit("client1").await.is_ok());
        }

        let result = manager.check_rate_limit("client1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_audit_logging() {
        let manager = SecurityManager::default();
        let service_id = ServiceId("service1".to_string());

        manager
            .log_audit(&service_id, "login", "user1", "success", HashMap::new())
            .await
            .unwrap();

        assert_eq!(manager.audit_log_count(), 1);
    }

    #[tokio::test]
    async fn test_block_ip() {
        let manager = SecurityManager::default();

        manager.block_ip("192.168.1.1").await.unwrap();
        let is_blocked = manager.is_ip_blocked("192.168.1.1").await.unwrap();
        assert!(is_blocked);
    }

    #[tokio::test]
    async fn test_unblock_ip() {
        let manager = SecurityManager::default();

        manager.block_ip("192.168.1.1").await.unwrap();
        manager.unblock_ip("192.168.1.1").await.unwrap();

        let is_blocked = manager.is_ip_blocked("192.168.1.1").await.unwrap();
        assert!(!is_blocked);
    }

    #[tokio::test]
    async fn test_reset_rate_limits() {
        let policy = SecurityPolicy {
            rate_limit_requests_per_sec: 2,
            ..Default::default()
        };
        let manager = SecurityManager::new(policy);

        manager.check_rate_limit("client1").await.unwrap();
        manager.reset_rate_limits().await.unwrap();

        let result = manager.check_rate_limit("client1").await;
        assert!(result.is_ok());
    }
}
