use crate::{DnsError, DnsResult, DnsRecord, DomainName, ZoneId};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct DynamicDnsManager {
    authorized_clients: Arc<DashMap<String, Vec<String>>>,
    update_history: Arc<DashMap<String, chrono::DateTime<Utc>>>,
}

impl DynamicDnsManager {
    pub fn new() -> Self {
        Self {
            authorized_clients: Arc::new(DashMap::new()),
            update_history: Arc::new(DashMap::new()),
        }
    }

    pub async fn authorize_client(
        &self,
        client_id: &str,
        allowed_zones: Vec<String>,
    ) -> DnsResult<()> {
        self.authorized_clients
            .insert(client_id.to_string(), allowed_zones);
        Ok(())
    }

    pub async fn revoke_client(&self, client_id: &str) -> DnsResult<()> {
        self.authorized_clients.remove(client_id);
        Ok(())
    }

    pub async fn is_authorized(&self, client_id: &str, zone: &DomainName) -> DnsResult<bool> {
        if let Some(entry) = self.authorized_clients.get(client_id) {
            Ok(entry.contains(&zone.0))
        } else {
            Ok(false)
        }
    }

    pub async fn apply_dynamic_update(
        &self,
        zone_id: &ZoneId,
        zone_name: &DomainName,
        client_id: &str,
        _records: Vec<DnsRecord>,
    ) -> DnsResult<()> {
        if !self.is_authorized(client_id, zone_name).await? {
            return Err(DnsError::DynamicUpdateFailed(
                format!("Client {} not authorized for zone {}", client_id, zone_name.0)
            ));
        }

        self.update_history
            .insert(zone_id.0.to_string(), Utc::now());

        Ok(())
    }

    pub async fn get_last_update(&self, zone_id: &ZoneId) -> DnsResult<Option<chrono::DateTime<Utc>>> {
        Ok(self
            .update_history
            .get(&zone_id.0.to_string())
            .map(|entry| *entry))
    }

    pub async fn get_update_count(&self) -> DnsResult<u64> {
        Ok(self.update_history.len() as u64)
    }

    pub fn authorized_client_count(&self) -> usize {
        self.authorized_clients.len()
    }
}

impl Default for DynamicDnsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_authorize_client() {
        let manager = DynamicDnsManager::new();
        let zones = vec!["example.com".to_string()];

        manager
            .authorize_client("client-1", zones)
            .await
            .unwrap();

        assert_eq!(manager.authorized_client_count(), 1);
    }

    #[tokio::test]
    async fn test_is_authorized() {
        let manager = DynamicDnsManager::new();
        let zones = vec!["example.com".to_string()];

        manager
            .authorize_client("client-1", zones)
            .await
            .unwrap();

        let authorized = manager
            .is_authorized("client-1", &DomainName("example.com".to_string()))
            .await
            .unwrap();

        assert!(authorized);
    }

    #[tokio::test]
    async fn test_unauthorized_client() {
        let manager = DynamicDnsManager::new();

        let authorized = manager
            .is_authorized("unknown-client", &DomainName("example.com".to_string()))
            .await
            .unwrap();

        assert!(!authorized);
    }

    #[tokio::test]
    async fn test_revoke_client() {
        let manager = DynamicDnsManager::new();
        let zones = vec!["example.com".to_string()];

        manager
            .authorize_client("client-1", zones)
            .await
            .unwrap();

        assert_eq!(manager.authorized_client_count(), 1);

        manager.revoke_client("client-1").await.unwrap();
        assert_eq!(manager.authorized_client_count(), 0);
    }

    #[tokio::test]
    async fn test_apply_dynamic_update_unauthorized() {
        let manager = DynamicDnsManager::new();
        let zone_id = ZoneId(uuid::Uuid::new_v4());
        let zone_name = DomainName("example.com".to_string());

        let result = manager
            .apply_dynamic_update(&zone_id, &zone_name, "unauthorized", vec![])
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_last_update() {
        use crate::RecordData;
        use std::net::Ipv4Addr;

        let manager = DynamicDnsManager::new();
        let zone_id = ZoneId(uuid::Uuid::new_v4());
        let zone_name = DomainName("example.com".to_string());

        manager
            .authorize_client("client-1", vec!["example.com".to_string()])
            .await
            .unwrap();

        let record = DnsRecord::new(
            "www".to_string(),
            crate::RecordType::A,
            RecordData::A(Ipv4Addr::new(192, 0, 2, 1)),
            3600,
        );

        manager
            .apply_dynamic_update(&zone_id, &zone_name, "client-1", vec![record])
            .await
            .unwrap();

        let last_update = manager.get_last_update(&zone_id).await.unwrap();
        assert!(last_update.is_some());
    }
}
