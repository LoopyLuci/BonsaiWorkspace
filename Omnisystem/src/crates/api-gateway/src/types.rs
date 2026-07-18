use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// A path registered with the [`crate::ApiGateway`], along with the
/// downstream service it forwards to and the HTTP methods it accepts.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Route {
    pub path: String,
    pub target_service: String,
    pub methods: Vec<String>,
}
