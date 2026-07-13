use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacyPolicy {
    pub policy_id: String,
    pub data_types: Vec<String>,
    pub retention_days: u32,
    pub requires_consent: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserConsent {
    pub user_id: String,
    pub consent_type: String,
    pub given_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DlpRule {
    pub rule_id: String,
    pub pattern: String,
    pub action: String,
}
