use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_by: String,
}

impl Record {
    pub fn new(created_by: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            created_by: created_by.clone(),
            updated_by: created_by,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateRequest {
    pub created_by: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateRequest {
    pub updated_by: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListResponse {
    pub items: Vec<Record>,
    pub count: usize,
}
