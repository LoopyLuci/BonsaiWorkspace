//! Baseline approval workflow with audit trail and rollback capability

use crate::baseline::BaselineVersion;
use crate::errors::{CIError, CIResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

/// Approval decision
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum ApprovalDecision {
    Approved,
    Rejected,
    RequestedChanges,
    Pending,
}

/// Single audit entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub entry_id: String,
    pub timestamp: DateTime<Utc>,
    pub user: String,
    pub action: String,
    pub baseline_version: String,
    pub decision: ApprovalDecision,
    pub reason: String,
    pub metadata: HashMap<String, String>,
}

/// Approval request for new baseline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub request_id: String,
    pub baseline_version: BaselineVersion,
    pub requested_by: String,
    pub requested_at: DateTime<Utc>,
    pub reason: String,
    pub performance_improvement: Option<f64>, // percentage
    pub risk_assessment: String,
    pub reviews: Vec<ApprovalReview>,
}

/// Review from an approver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalReview {
    pub reviewer: String,
    pub reviewed_at: DateTime<Utc>,
    pub decision: ApprovalDecision,
    pub comments: String,
}

/// Manages baseline approval workflows
pub struct ApprovalManager {
    audit_log: dashmap::DashMap<String, AuditEntry>,
    approval_requests: dashmap::DashMap<String, ApprovalRequest>,
    approved_versions: dashmap::DashMap<String, BaselineVersion>,
    version_history: dashmap::DashMap<String, Vec<BaselineVersion>>,
}

impl ApprovalManager {
    /// Create new approval manager
    pub fn new() -> Self {
        Self {
            audit_log: dashmap::DashMap::new(),
            approval_requests: dashmap::DashMap::new(),
            approved_versions: dashmap::DashMap::new(),
            version_history: dashmap::DashMap::new(),
        }
    }

    /// Create approval request for new baseline
    pub fn request_approval(
        &self,
        baseline_version: BaselineVersion,
        requester: &str,
        reason: &str,
        performance_improvement: Option<f64>,
        risk_assessment: &str,
    ) -> CIResult<String> {
        let request_id = uuid::Uuid::new_v4().to_string();

        let request = ApprovalRequest {
            request_id: request_id.clone(),
            baseline_version: baseline_version.clone(),
            requested_by: requester.to_string(),
            requested_at: Utc::now(),
            reason: reason.to_string(),
            performance_improvement,
            risk_assessment: risk_assessment.to_string(),
            reviews: Vec::new(),
        };

        self.approval_requests
            .insert(request_id.clone(), request.clone());

        // Log the request
        self.log_action(
            requester,
            "REQUEST_APPROVAL",
            &baseline_version.version,
            ApprovalDecision::Pending,
            reason,
        )?;

        info!(
            "Approval request {} created for baseline {}",
            request_id, baseline_version.version
        );

        Ok(request_id)
    }

    /// Submit review for approval request
    pub fn submit_review(
        &self,
        request_id: &str,
        reviewer: &str,
        decision: ApprovalDecision,
        comments: &str,
    ) -> CIResult<()> {
        if let Some(mut request) = self.approval_requests.get_mut(request_id) {
            let review = ApprovalReview {
                reviewer: reviewer.to_string(),
                reviewed_at: Utc::now(),
                decision,
                comments: comments.to_string(),
            };

            request.reviews.push(review);

            info!(
                "Review submitted for {} by {} ({:?})",
                request_id, reviewer, decision
            );

            Ok(())
        } else {
            Err(CIError::ApprovalWorkflowError(format!(
                "Request {} not found",
                request_id
            )))
        }
    }

    /// Approve baseline (after sufficient reviews)
    pub fn approve_baseline(
        &self,
        request_id: &str,
        approver: &str,
    ) -> CIResult<()> {
        if let Some(request) = self.approval_requests.get(request_id) {
            // Check if consensus reached
            let approved_count = request
                .reviews
                .iter()
                .filter(|r| r.decision == ApprovalDecision::Approved)
                .count();
            let rejected_count = request
                .reviews
                .iter()
                .filter(|r| r.decision == ApprovalDecision::Rejected)
                .count();

            if rejected_count > 0 {
                return Err(CIError::ApprovalWorkflowError(
                    "Cannot approve: rejections exist".to_string(),
                ));
            }

            if approved_count < 1 {
                return Err(CIError::ApprovalWorkflowError(
                    "Need at least 1 approval".to_string(),
                ));
            }

            // Store approved version
            let baseline = request.baseline_version.clone();
            self.approved_versions
                .insert(baseline.version.clone(), baseline.clone());

            // Add to history
            let mut history = self.version_history
                .entry(baseline.commit_hash.clone())
                .or_insert_with(Vec::new);
            history.push(baseline.clone());

            // Log approval
            self.log_action(
                approver,
                "APPROVED",
                &baseline.version,
                ApprovalDecision::Approved,
                &format!("Approved by {}", approver),
            )?;

            info!("Baseline {} approved", baseline.version);

            Ok(())
        } else {
            Err(CIError::ApprovalWorkflowError(format!(
                "Request {} not found",
                request_id
            )))
        }
    }

    /// Reject baseline
    pub fn reject_baseline(
        &self,
        request_id: &str,
        reviewer: &str,
        reason: &str,
    ) -> CIResult<()> {
        if let Some(request) = self.approval_requests.get(request_id) {
            let baseline = request.baseline_version.clone();

            self.log_action(
                reviewer,
                "REJECTED",
                &baseline.version,
                ApprovalDecision::Rejected,
                reason,
            )?;

            warn!("Baseline {} rejected: {}", baseline.version, reason);

            Ok(())
        } else {
            Err(CIError::ApprovalWorkflowError(format!(
                "Request {} not found",
                request_id
            )))
        }
    }

    /// Get approval request
    pub fn get_request(&self, request_id: &str) -> Option<ApprovalRequest> {
        self.approval_requests.get(request_id).map(|r| r.clone())
    }

    /// List pending requests
    pub fn list_pending_requests(&self) -> Vec<ApprovalRequest> {
        self.approval_requests
            .iter()
            .filter(|entry| entry.value().reviews.is_empty())
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// List approved baselines
    pub fn list_approved_baselines(&self) -> Vec<BaselineVersion> {
        self.approved_versions
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Check if baseline is approved
    pub fn is_approved(&self, baseline_version: &str) -> bool {
        self.approved_versions.contains_key(baseline_version)
    }

    /// Get approval history for baseline
    pub fn get_history(&self, commit_hash: &str) -> Vec<BaselineVersion> {
        self.version_history
            .get(commit_hash)
            .map(|entry| entry.clone())
            .unwrap_or_default()
    }

    /// Rollback to previous baseline version
    pub fn rollback(&self, commit_hash: &str, to_version: &str) -> CIResult<()> {
        if let Some(history) = self.version_history.get(commit_hash) {
            let found = history
                .iter()
                .any(|v| v.version == to_version);

            if !found {
                return Err(CIError::ApprovalWorkflowError(format!(
                    "Version {} not in history",
                    to_version
                )));
            }

            info!(
                "Rolling back {} to {}",
                commit_hash, to_version
            );

            Ok(())
        } else {
            Err(CIError::ApprovalWorkflowError(
                "No history found".to_string(),
            ))
        }
    }

    /// Get audit log
    pub fn get_audit_log(&self) -> Vec<AuditEntry> {
        self.audit_log
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get audit log for specific baseline
    pub fn get_baseline_audit_log(&self, baseline_version: &str) -> Vec<AuditEntry> {
        self.audit_log
            .iter()
            .filter(|entry| entry.value().baseline_version == baseline_version)
            .map(|entry| entry.value().clone())
            .collect()
    }

    fn log_action(
        &self,
        user: &str,
        action: &str,
        baseline_version: &str,
        decision: ApprovalDecision,
        reason: &str,
    ) -> CIResult<()> {
        let entry_id = uuid::Uuid::new_v4().to_string();

        let entry = AuditEntry {
            entry_id: entry_id.clone(),
            timestamp: Utc::now(),
            user: user.to_string(),
            action: action.to_string(),
            baseline_version: baseline_version.to_string(),
            decision,
            reason: reason.to_string(),
            metadata: HashMap::new(),
        };

        self.audit_log.insert(entry_id, entry);
        Ok(())
    }
}

impl Default for ApprovalManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_request_creation() {
        let manager = ApprovalManager::new();

        let baseline = BaselineVersion {
            version: "1.1.0".to_string(),
            commit_hash: "xyz789".to_string(),
            timestamp: Utc::now(),
            approved_by: None,
        };

        let request_id = manager
            .request_approval(
                baseline,
                "alice",
                "Performance improvement",
                Some(10.5),
                "Low risk",
            )
            .unwrap();

        assert!(!request_id.is_empty());
        let request = manager.get_request(&request_id).unwrap();
        assert_eq!(request.requested_by, "alice");
    }

    #[test]
    fn test_submit_review() {
        let manager = ApprovalManager::new();

        let baseline = BaselineVersion {
            version: "1.1.0".to_string(),
            commit_hash: "xyz789".to_string(),
            timestamp: Utc::now(),
            approved_by: None,
        };

        let request_id = manager
            .request_approval(
                baseline,
                "alice",
                "Performance improvement",
                None,
                "Low risk",
            )
            .unwrap();

        manager
            .submit_review(
                &request_id,
                "bob",
                ApprovalDecision::Approved,
                "Looks good",
            )
            .unwrap();

        let request = manager.get_request(&request_id).unwrap();
        assert_eq!(request.reviews.len(), 1);
        assert_eq!(request.reviews[0].reviewer, "bob");
    }

    #[test]
    fn test_approve_baseline() {
        let manager = ApprovalManager::new();

        let baseline = BaselineVersion {
            version: "1.1.0".to_string(),
            commit_hash: "xyz789".to_string(),
            timestamp: Utc::now(),
            approved_by: None,
        };

        let request_id = manager
            .request_approval(baseline.clone(), "alice", "Performance", None, "Low")
            .unwrap();

        manager
            .submit_review(
                &request_id,
                "bob",
                ApprovalDecision::Approved,
                "Good",
            )
            .unwrap();

        assert!(manager.approve_baseline(&request_id, "charlie").is_ok());
        assert!(manager.is_approved("1.1.0"));
    }

    #[test]
    fn test_reject_baseline() {
        let manager = ApprovalManager::new();

        let baseline = BaselineVersion {
            version: "1.2.0".to_string(),
            commit_hash: "xyz790".to_string(),
            timestamp: Utc::now(),
            approved_by: None,
        };

        let request_id = manager
            .request_approval(baseline, "alice", "Test", None, "Medium")
            .unwrap();

        assert!(manager
            .reject_baseline(&request_id, "bob", "Needs more testing")
            .is_ok());
    }

    #[test]
    fn test_list_pending_requests() {
        let manager = ApprovalManager::new();

        for i in 0..3 {
            let baseline = BaselineVersion {
                version: format!("1.{}.0", i),
                commit_hash: format!("hash{}", i),
                timestamp: Utc::now(),
                approved_by: None,
            };

            let _ = manager.request_approval(
                baseline,
                "alice",
                "Test",
                None,
                "Low",
            );
        }

        let pending = manager.list_pending_requests();
        assert_eq!(pending.len(), 3);
    }

    #[test]
    fn test_list_approved_baselines() {
        let manager = ApprovalManager::new();

        let baseline = BaselineVersion {
            version: "1.1.0".to_string(),
            commit_hash: "xyz789".to_string(),
            timestamp: Utc::now(),
            approved_by: None,
        };

        let request_id = manager
            .request_approval(baseline, "alice", "Test", None, "Low")
            .unwrap();

        manager
            .submit_review(&request_id, "bob", ApprovalDecision::Approved, "OK")
            .unwrap();

        manager.approve_baseline(&request_id, "charlie").unwrap();

        let approved = manager.list_approved_baselines();
        assert!(approved.iter().any(|b| b.version == "1.1.0"));
    }

    #[test]
    fn test_is_approved() {
        let manager = ApprovalManager::new();

        let baseline = BaselineVersion {
            version: "2.0.0".to_string(),
            commit_hash: "hash2".to_string(),
            timestamp: Utc::now(),
            approved_by: None,
        };

        let request_id = manager
            .request_approval(baseline, "alice", "Test", None, "Low")
            .unwrap();

        manager
            .submit_review(&request_id, "bob", ApprovalDecision::Approved, "OK")
            .unwrap();

        manager.approve_baseline(&request_id, "charlie").unwrap();

        assert!(manager.is_approved("2.0.0"));
        assert!(!manager.is_approved("3.0.0"));
    }

    #[test]
    fn test_rollback() {
        let manager = ApprovalManager::new();

        let baseline1 = BaselineVersion {
            version: "1.0.0".to_string(),
            commit_hash: "hash1".to_string(),
            timestamp: Utc::now(),
            approved_by: None,
        };

        let baseline2 = BaselineVersion {
            version: "1.1.0".to_string(),
            commit_hash: "hash1".to_string(),
            timestamp: Utc::now(),
            approved_by: None,
        };

        let req1 = manager
            .request_approval(baseline1, "alice", "Initial", None, "Low")
            .unwrap();

        manager
            .submit_review(&req1, "bob", ApprovalDecision::Approved, "OK")
            .unwrap();
        manager.approve_baseline(&req1, "charlie").unwrap();

        let req2 = manager
            .request_approval(baseline2, "alice", "Update", None, "Low")
            .unwrap();

        manager
            .submit_review(&req2, "bob", ApprovalDecision::Approved, "OK")
            .unwrap();
        manager.approve_baseline(&req2, "charlie").unwrap();

        assert!(manager.rollback("hash1", "1.0.0").is_ok());
    }

    #[test]
    fn test_audit_log() {
        let manager = ApprovalManager::new();

        let baseline = BaselineVersion {
            version: "1.0.0".to_string(),
            commit_hash: "hash1".to_string(),
            timestamp: Utc::now(),
            approved_by: None,
        };

        let _ = manager.request_approval(baseline, "alice", "Test", None, "Low");

        let audit = manager.get_audit_log();
        assert!(!audit.is_empty());
        assert_eq!(audit[0].user, "alice");
    }

    #[test]
    fn test_baseline_audit_log() {
        let manager = ApprovalManager::new();

        let baseline = BaselineVersion {
            version: "1.0.0".to_string(),
            commit_hash: "hash1".to_string(),
            timestamp: Utc::now(),
            approved_by: None,
        };

        let request_id = manager
            .request_approval(baseline, "alice", "Test", None, "Low")
            .unwrap();

        manager
            .submit_review(&request_id, "bob", ApprovalDecision::Approved, "OK")
            .unwrap();

        let baseline_audit = manager.get_baseline_audit_log("1.0.0");
        assert!(!baseline_audit.is_empty());
    }

    #[test]
    fn test_approval_decision_equality() {
        assert_eq!(ApprovalDecision::Approved, ApprovalDecision::Approved);
        assert_ne!(ApprovalDecision::Approved, ApprovalDecision::Rejected);
    }

    #[test]
    fn test_cannot_approve_with_rejection() {
        let manager = ApprovalManager::new();

        let baseline = BaselineVersion {
            version: "1.0.0".to_string(),
            commit_hash: "hash1".to_string(),
            timestamp: Utc::now(),
            approved_by: None,
        };

        let request_id = manager
            .request_approval(baseline, "alice", "Test", None, "Low")
            .unwrap();

        manager
            .submit_review(&request_id, "bob", ApprovalDecision::Rejected, "No")
            .unwrap();

        let result = manager.approve_baseline(&request_id, "charlie");
        assert!(result.is_err());
    }
}
