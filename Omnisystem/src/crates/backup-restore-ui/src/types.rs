//! Data types
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// ID
    pub id: String,
    /// Created at
    pub created_at: DateTime<Utc>,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: Utc::now(),
        }
    }
}
