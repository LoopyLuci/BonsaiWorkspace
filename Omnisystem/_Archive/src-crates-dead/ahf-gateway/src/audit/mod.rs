//! Audit logging for the Anti-Hallucination Gateway

use crate::error::{GatewayError, GatewayResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Entry ID
    pub id: Uuid,
    /// Request ID being audited
    pub request_id: Uuid,
    /// Timestamp of decision
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Decision made (Accept/Reject/Escalate)
    pub decision: String,
    /// Grounding score
    pub grounding_score: f64,
    /// Verification valid flag
    pub verification_valid: bool,
    /// Model confidence
    pub model_confidence: f64,
    /// Bias score
    pub bias_score: f64,
    /// Decision reasoning
    pub reasoning: String,
    /// Model ID
    pub model_id: String,
    /// Optional user ID
    pub user_id: Option<String>,
    /// Optional session ID
    pub session_id: Option<Uuid>,
    /// BLAKE3 hash of entry for integrity
    pub hash: String,
    /// Previous entry hash (for chain integrity)
    pub previous_hash: Option<String>,
}

impl AuditEntry {
    pub fn new(
        request_id: Uuid,
        decision: String,
        grounding_score: f64,
        verification_valid: bool,
        model_confidence: f64,
        bias_score: f64,
        reasoning: String,
        model_id: String,
    ) -> Self {
        let mut entry = Self {
            id: Uuid::new_v4(),
            request_id,
            timestamp: chrono::Utc::now(),
            decision,
            grounding_score,
            verification_valid,
            model_confidence,
            bias_score,
            reasoning,
            model_id,
            user_id: None,
            session_id: None,
            hash: String::new(),
            previous_hash: None,
        };

        entry.hash = entry.compute_hash();
        entry
    }

    fn compute_hash(&self) -> String {
        let data = format!(
            "{}:{}:{}:{}:{}:{}:{}",
            self.request_id,
            self.timestamp.timestamp(),
            self.decision,
            self.grounding_score,
            self.model_confidence,
            self.bias_score,
            self.model_id
        );
        blake3::hash(data.as_bytes()).to_hex().to_string()
    }

    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self.hash = self.compute_hash();
        self
    }

    pub fn with_session_id(mut self, session_id: Uuid) -> Self {
        self.session_id = Some(session_id);
        self.hash = self.compute_hash();
        self
    }

    /// Verify the integrity of this entry
    pub fn verify_integrity(&self) -> bool {
        self.hash == self.compute_hash()
    }
}

/// Immutable audit log
pub struct AuditLog {
    entries: Arc<RwLock<Vec<AuditEntry>>>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(vec![])),
        }
    }

    /// Add an entry to the audit log
    pub async fn add(&self, mut entry: AuditEntry) -> GatewayResult<()> {
        let mut entries = self.entries.write().await;

        // Set previous hash for chain integrity
        if let Some(last_entry) = entries.last() {
            entry.previous_hash = Some(last_entry.hash.clone());
        }

        // Update hash with previous hash
        entry.hash = entry.compute_hash();

        entries.push(entry);
        Ok(())
    }

    /// Get all entries
    pub async fn get_all(&self) -> Vec<AuditEntry> {
        self.entries.read().await.clone()
    }

    /// Get entries by request ID
    pub async fn get_by_request_id(&self, request_id: Uuid) -> Vec<AuditEntry> {
        self.entries
            .read()
            .await
            .iter()
            .filter(|e| e.request_id == request_id)
            .cloned()
            .collect()
    }

    /// Get entries by model ID
    pub async fn get_by_model_id(&self, model_id: &str) -> Vec<AuditEntry> {
        self.entries
            .read()
            .await
            .iter()
            .filter(|e| e.model_id == model_id)
            .cloned()
            .collect()
    }

    /// Get entries by user ID
    pub async fn get_by_user_id(&self, user_id: &str) -> Vec<AuditEntry> {
        self.entries
            .read()
            .await
            .iter()
            .filter(|e| e.user_id.as_deref() == Some(user_id))
            .cloned()
            .collect()
    }

    /// Get entries in time range
    pub async fn get_by_time_range(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Vec<AuditEntry> {
        self.entries
            .read()
            .await
            .iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .cloned()
            .collect()
    }

    /// Verify chain integrity
    pub async fn verify_chain_integrity(&self) -> bool {
        let entries = self.entries.read().await;

        for i in 0..entries.len() {
            let entry = &entries[i];

            // Verify this entry's hash
            if !entry.verify_integrity() {
                return false;
            }

            // Verify chain link
            if i > 0 {
                let prev = &entries[i - 1];
                if entry.previous_hash.as_ref() != Some(&prev.hash) {
                    return false;
                }
            } else if entry.previous_hash.is_some() {
                return false;
            }
        }

        true
    }

    /// Get entry count
    pub async fn count(&self) -> usize {
        self.entries.read().await.len()
    }

    /// Clear all entries (for testing)
    #[cfg(test)]
    pub async fn clear(&self) {
        self.entries.write().await.clear();
    }
}

impl Clone for AuditLog {
    fn clone(&self) -> Self {
        Self {
            entries: Arc::clone(&self.entries),
        }
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_entry_creation() {
        let entry = AuditEntry::new(
            Uuid::new_v4(),
            "Accept".to_string(),
            0.85,
            true,
            0.90,
            0.1,
            "All checks passed".to_string(),
            "gpt-4".to_string(),
        );

        assert_eq!(entry.decision, "Accept");
        assert_eq!(entry.model_id, "gpt-4");
        assert!(!entry.hash.is_empty());
    }

    #[test]
    fn test_audit_entry_integrity() {
        let entry = AuditEntry::new(
            Uuid::new_v4(),
            "Accept".to_string(),
            0.85,
            true,
            0.90,
            0.1,
            "All checks passed".to_string(),
            "gpt-4".to_string(),
        );

        assert!(entry.verify_integrity());
    }

    #[test]
    fn test_audit_entry_builder() {
        let user_id = "user123".to_string();
        let session_id = Uuid::new_v4();

        let entry = AuditEntry::new(
            Uuid::new_v4(),
            "Accept".to_string(),
            0.85,
            true,
            0.90,
            0.1,
            "Passed".to_string(),
            "gpt-4".to_string(),
        )
        .with_user_id(user_id.clone())
        .with_session_id(session_id);

        assert_eq!(entry.user_id, Some(user_id));
        assert_eq!(entry.session_id, Some(session_id));
    }

    #[tokio::test]
    async fn test_audit_log_add_entry() {
        let log = AuditLog::new();
        let entry = AuditEntry::new(
            Uuid::new_v4(),
            "Accept".to_string(),
            0.85,
            true,
            0.90,
            0.1,
            "All checks passed".to_string(),
            "gpt-4".to_string(),
        );

        assert!(log.add(entry).await.is_ok());
        assert_eq!(log.count().await, 1);
    }

    #[tokio::test]
    async fn test_audit_log_get_all() {
        let log = AuditLog::new();
        let req_id = Uuid::new_v4();

        let entry1 = AuditEntry::new(
            req_id,
            "Accept".to_string(),
            0.85,
            true,
            0.90,
            0.1,
            "Passed".to_string(),
            "gpt-4".to_string(),
        );

        let entry2 = AuditEntry::new(
            Uuid::new_v4(),
            "Reject".to_string(),
            0.4,
            false,
            0.5,
            0.8,
            "Failed".to_string(),
            "gpt-3".to_string(),
        );

        log.add(entry1).await.unwrap();
        log.add(entry2).await.unwrap();

        let all = log.get_all().await;
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_audit_log_get_by_request_id() {
        let log = AuditLog::new();
        let req_id = Uuid::new_v4();

        let entry = AuditEntry::new(
            req_id,
            "Accept".to_string(),
            0.85,
            true,
            0.90,
            0.1,
            "Passed".to_string(),
            "gpt-4".to_string(),
        );

        log.add(entry).await.unwrap();

        let entries = log.get_by_request_id(req_id).await;
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test]
    async fn test_audit_log_get_by_model_id() {
        let log = AuditLog::new();

        let entry1 = AuditEntry::new(
            Uuid::new_v4(),
            "Accept".to_string(),
            0.85,
            true,
            0.90,
            0.1,
            "Passed".to_string(),
            "gpt-4".to_string(),
        );

        let entry2 = AuditEntry::new(
            Uuid::new_v4(),
            "Accept".to_string(),
            0.80,
            true,
            0.85,
            0.15,
            "Passed".to_string(),
            "gpt-4".to_string(),
        );

        let entry3 = AuditEntry::new(
            Uuid::new_v4(),
            "Accept".to_string(),
            0.75,
            true,
            0.80,
            0.2,
            "Passed".to_string(),
            "claude".to_string(),
        );

        log.add(entry1).await.unwrap();
        log.add(entry2).await.unwrap();
        log.add(entry3).await.unwrap();

        let gpt4_entries = log.get_by_model_id("gpt-4").await;
        assert_eq!(gpt4_entries.len(), 2);

        let claude_entries = log.get_by_model_id("claude").await;
        assert_eq!(claude_entries.len(), 1);
    }

    #[tokio::test]
    async fn test_audit_log_chain_integrity() {
        let log = AuditLog::new();

        let entry1 = AuditEntry::new(
            Uuid::new_v4(),
            "Accept".to_string(),
            0.85,
            true,
            0.90,
            0.1,
            "Passed".to_string(),
            "gpt-4".to_string(),
        );

        let entry2 = AuditEntry::new(
            Uuid::new_v4(),
            "Reject".to_string(),
            0.4,
            false,
            0.5,
            0.8,
            "Failed".to_string(),
            "gpt-3".to_string(),
        );

        log.add(entry1).await.unwrap();
        log.add(entry2).await.unwrap();

        assert!(log.verify_chain_integrity().await);
    }

    #[tokio::test]
    async fn test_audit_log_by_user_id() {
        let log = AuditLog::new();

        let entry = AuditEntry::new(
            Uuid::new_v4(),
            "Accept".to_string(),
            0.85,
            true,
            0.90,
            0.1,
            "Passed".to_string(),
            "gpt-4".to_string(),
        )
        .with_user_id("user123".to_string());

        log.add(entry).await.unwrap();

        let entries = log.get_by_user_id("user123").await;
        assert_eq!(entries.len(), 1);
    }
}
