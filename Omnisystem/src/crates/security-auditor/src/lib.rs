//! security-auditor: findings/policy tracking, compliance frameworks,
//! reporting, and a small rule engine for common security checks.

pub mod auditor;
pub mod compliance;
pub mod error;
pub mod reporting;
pub mod rules;

pub use auditor::{FindingStatus, SecurityAuditor, SecurityFinding, SecurityPolicy, Severity};
pub use compliance::{ComplianceFramework, ComplianceManager};
pub use error::{AuditError, Result};
pub use reporting::{AuditReport, ReportGenerator};
pub use rules::RuleEngine;
