use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Deployment {
    pub deployment_id: String,
    pub version: String,
    pub status: String,
    pub deployed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Release {
    pub release_id: String,
    pub version: String,
    pub release_notes: String,
}
