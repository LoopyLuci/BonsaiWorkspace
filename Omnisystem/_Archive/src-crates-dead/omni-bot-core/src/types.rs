//! Core types used throughout Omni-Bot

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Unique request ID for tracing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(pub Uuid);

impl RequestId {
    pub fn new() -> Self {
        RequestId(Uuid::new_v4())
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

/// User identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub String);

/// Session identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

impl SessionId {
    pub fn new() -> Self {
        SessionId(Uuid::new_v4())
    }
}

/// Resource specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSpec {
    pub memory_mb: u32,
    pub cpu_cores: u32,
    pub cpu_percent_max: u32,
    pub iops_limit: u32,
}

impl Default for ResourceSpec {
    fn default() -> Self {
        Self {
            memory_mb: 512,
            cpu_cores: 2,
            cpu_percent_max: 100,
            iops_limit: 1000,
        }
    }
}

/// API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(request_id: String, data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            request_id,
            timestamp: Utc::now(),
        }
    }
    
    pub fn error(request_id: String, error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            request_id,
            timestamp: Utc::now(),
        }
    }
}

/// Generic metadata map
pub type Metadata = HashMap<String, serde_json::Value>;
