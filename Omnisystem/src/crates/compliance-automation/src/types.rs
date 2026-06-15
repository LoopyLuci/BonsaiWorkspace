use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompliancePolicy {
    pub policy_id: Uuid,
    pub framework: ComplianceFramework,
    pub policy_name: String,
    pub requirement: String,
    pub created_at: DateTime<Utc>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum ComplianceFramework {
    SOC2,
    HIPAA,
    GDPR,
    PCI_DSS,
    ISO27001,
    Custom(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyEvaluation {
    pub evaluation_id: Uuid,
    pub policy_id: Uuid,
    pub evaluated_at: DateTime<Utc>,
    pub compliant: bool,
    pub score: f64,
    pub findings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComplianceViolation {
    pub violation_id: Uuid,
    pub policy_id: Uuid,
    pub violation_type: String,
    pub severity: ViolationSeverity,
    pub detected_at: DateTime<Utc>,
    pub remediation_due: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub report_id: Uuid,
    pub framework: ComplianceFramework,
    pub generated_at: DateTime<Utc>,
    pub total_policies: u32,
    pub compliant_count: u32,
    pub violation_count: u32,
    pub overall_score: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegulatoryRequirement {
    pub requirement_id: Uuid,
    pub framework: ComplianceFramework,
    pub requirement_code: String,
    pub description: String,
    pub last_audited: Option<DateTime<Utc>>,
}
