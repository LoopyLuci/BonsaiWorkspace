use crate::{PrivacyError, PrivacyResult, UserConsent};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct GdprManager {
    consents: Arc<DashMap<String, UserConsent>>,
}

impl GdprManager {
    pub fn new() -> Self {
        Self {
            consents: Arc::new(DashMap::new()),
        }
    }

    pub async fn record_consent(&self, consent: &UserConsent) -> PrivacyResult<()> {
        self.consents.insert(consent.user_id.clone(), consent.clone());
        Ok(())
    }

    pub async fn check_consent(&self, user_id: &str) -> PrivacyResult<bool> {
        if let Some(consent) = self.consents.get(user_id) {
            if let Some(expires_at) = consent.expires_at {
                Ok(expires_at > Utc::now())
            } else {
                Ok(true)
            }
        } else {
            Err(PrivacyError::ConsentMissing)
        }
    }

    pub async fn delete_user_data(&self, user_id: &str) -> PrivacyResult<()> {
        if self.consents.remove(user_id).is_some() {
            Ok(())
        } else {
            Err(PrivacyError::DeletionFailed)
        }
    }

    pub fn consent_count(&self) -> usize {
        self.consents.len()
    }
}

impl Default for GdprManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_consent() {
        let manager = GdprManager::new();
        let consent = UserConsent {
            user_id: "u1".to_string(),
            consent_type: "data_processing".to_string(),
            given_at: Utc::now(),
            expires_at: None,
        };

        manager.record_consent(&consent).await.unwrap();
        assert_eq!(manager.consent_count(), 1);
    }

    #[tokio::test]
    async fn test_check_consent() {
        let manager = GdprManager::new();
        let consent = UserConsent {
            user_id: "u1".to_string(),
            consent_type: "data_processing".to_string(),
            given_at: Utc::now(),
            expires_at: None,
        };

        manager.record_consent(&consent).await.unwrap();
        let has_consent = manager.check_consent("u1").await.unwrap();
        assert!(has_consent);
    }

    #[tokio::test]
    async fn test_delete_user_data() {
        let manager = GdprManager::new();
        let consent = UserConsent {
            user_id: "u1".to_string(),
            consent_type: "data_processing".to_string(),
            given_at: Utc::now(),
            expires_at: None,
        };

        manager.record_consent(&consent).await.unwrap();
        manager.delete_user_data("u1").await.unwrap();
        assert_eq!(manager.consent_count(), 0);
    }
}
