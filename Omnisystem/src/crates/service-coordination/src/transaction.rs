use crate::{CoordinationError, CoordinationResult, DistributedTransaction, TransactionParticipant, TransactionPhase};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct TransactionManager {
    transactions: Arc<DashMap<String, DistributedTransaction>>,
}

impl TransactionManager {
    pub fn new() -> Self {
        Self {
            transactions: Arc::new(DashMap::new()),
        }
    }

    pub async fn begin_transaction(&self, transaction_id: &str, timeout_ms: u64) -> CoordinationResult<DistributedTransaction> {
        let transaction = DistributedTransaction {
            transaction_id: transaction_id.to_string(),
            phase: TransactionPhase::Prepare,
            participants: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            timeout_ms,
        };

        self.transactions.insert(transaction_id.to_string(), transaction.clone());
        Ok(transaction)
    }

    pub async fn register_participant(
        &self,
        transaction_id: &str,
        service_id: &str,
    ) -> CoordinationResult<()> {
        if let Some(mut txn) = self.transactions.get_mut(transaction_id) {
            let participant = TransactionParticipant {
                service_id: service_id.to_string(),
                timestamp: Utc::now(),
                prepared: false,
                ready_to_commit: false,
            };

            txn.participants.push(participant);
            txn.updated_at = Utc::now();
            Ok(())
        } else {
            Err(CoordinationError::TransactionNotFound)
        }
    }

    pub async fn mark_prepared(
        &self,
        transaction_id: &str,
        service_id: &str,
    ) -> CoordinationResult<()> {
        if let Some(mut txn) = self.transactions.get_mut(transaction_id) {
            if txn.phase != TransactionPhase::Prepare {
                return Err(CoordinationError::InvalidPhase);
            }

            for participant in &mut txn.participants {
                if participant.service_id == service_id {
                    participant.prepared = true;
                    break;
                }
            }

            txn.updated_at = Utc::now();
            Ok(())
        } else {
            Err(CoordinationError::TransactionNotFound)
        }
    }

    pub async fn can_commit(&self, transaction_id: &str) -> CoordinationResult<bool> {
        if let Some(txn) = self.transactions.get(transaction_id) {
            let all_prepared = txn.participants.iter().all(|p| p.prepared);
            Ok(all_prepared && !txn.participants.is_empty())
        } else {
            Err(CoordinationError::TransactionNotFound)
        }
    }

    pub async fn commit_transaction(&self, transaction_id: &str) -> CoordinationResult<()> {
        if let Some(mut txn) = self.transactions.get_mut(transaction_id) {
            if txn.phase != TransactionPhase::Prepare {
                return Err(CoordinationError::InvalidPhase);
            }

            let can_commit = txn.participants.iter().all(|p| p.prepared);
            if !can_commit {
                return Err(CoordinationError::InvalidPhase);
            }

            txn.phase = TransactionPhase::Commit;
            for participant in &mut txn.participants {
                participant.ready_to_commit = true;
            }

            txn.updated_at = Utc::now();
            Ok(())
        } else {
            Err(CoordinationError::TransactionNotFound)
        }
    }

    pub async fn complete_transaction(&self, transaction_id: &str) -> CoordinationResult<()> {
        if let Some(mut txn) = self.transactions.get_mut(transaction_id) {
            if txn.phase != TransactionPhase::Commit {
                return Err(CoordinationError::InvalidPhase);
            }

            txn.phase = TransactionPhase::Complete;
            txn.updated_at = Utc::now();
            Ok(())
        } else {
            Err(CoordinationError::TransactionNotFound)
        }
    }

    pub async fn rollback_transaction(&self, transaction_id: &str) -> CoordinationResult<()> {
        if let Some(mut txn) = self.transactions.get_mut(transaction_id) {
            txn.phase = TransactionPhase::Rollback;
            txn.updated_at = Utc::now();
            Ok(())
        } else {
            Err(CoordinationError::TransactionNotFound)
        }
    }

    pub async fn get_transaction(&self, transaction_id: &str) -> CoordinationResult<DistributedTransaction> {
        self.transactions
            .get(transaction_id)
            .map(|entry| entry.clone())
            .ok_or(CoordinationError::TransactionNotFound)
    }

    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }
}

impl Default for TransactionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_begin_transaction() {
        let manager = TransactionManager::new();
        let txn = manager.begin_transaction("txn-1", 5000).await.unwrap();

        assert_eq!(txn.transaction_id, "txn-1");
        assert_eq!(txn.phase, TransactionPhase::Prepare);
        assert_eq!(txn.timeout_ms, 5000);
    }

    #[tokio::test]
    async fn test_register_participant() {
        let manager = TransactionManager::new();
        manager.begin_transaction("txn-1", 5000).await.unwrap();

        manager.register_participant("txn-1", "service-1").await.unwrap();
        let txn = manager.get_transaction("txn-1").await.unwrap();

        assert_eq!(txn.participants.len(), 1);
    }

    #[tokio::test]
    async fn test_mark_prepared() {
        let manager = TransactionManager::new();
        manager.begin_transaction("txn-1", 5000).await.unwrap();
        manager.register_participant("txn-1", "service-1").await.unwrap();

        manager.mark_prepared("txn-1", "service-1").await.unwrap();
        let txn = manager.get_transaction("txn-1").await.unwrap();

        assert!(txn.participants[0].prepared);
    }

    #[tokio::test]
    async fn test_can_commit() {
        let manager = TransactionManager::new();
        manager.begin_transaction("txn-1", 5000).await.unwrap();
        manager.register_participant("txn-1", "service-1").await.unwrap();

        let can_commit = manager.can_commit("txn-1").await.unwrap();
        assert!(!can_commit);

        manager.mark_prepared("txn-1", "service-1").await.unwrap();
        let can_commit = manager.can_commit("txn-1").await.unwrap();
        assert!(can_commit);
    }

    #[tokio::test]
    async fn test_commit_transaction() {
        let manager = TransactionManager::new();
        manager.begin_transaction("txn-1", 5000).await.unwrap();
        manager.register_participant("txn-1", "service-1").await.unwrap();
        manager.mark_prepared("txn-1", "service-1").await.unwrap();

        manager.commit_transaction("txn-1").await.unwrap();
        let txn = manager.get_transaction("txn-1").await.unwrap();

        assert_eq!(txn.phase, TransactionPhase::Commit);
    }

    #[tokio::test]
    async fn test_rollback_transaction() {
        let manager = TransactionManager::new();
        manager.begin_transaction("txn-1", 5000).await.unwrap();

        manager.rollback_transaction("txn-1").await.unwrap();
        let txn = manager.get_transaction("txn-1").await.unwrap();

        assert_eq!(txn.phase, TransactionPhase::Rollback);
    }

    #[tokio::test]
    async fn test_transaction_not_found() {
        let manager = TransactionManager::new();
        let result = manager.get_transaction("nonexistent").await;

        assert!(result.is_err());
    }
}
