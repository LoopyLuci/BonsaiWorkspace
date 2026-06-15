//! Data types
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

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

/// Command interpretation from natural language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInterpretation {
    /// Action to perform (list, create, start, stop, etc.)
    pub action: String,
    /// Resource type (container, image, network, volume)
    pub resource_type: String,
    /// Specific resource name or ID
    pub resource_name: String,
    /// Additional parameters
    pub parameters: HashMap<String, String>,
    /// Confidence level (0.0-1.0)
    pub confidence: f64,
    /// Human-readable explanation
    pub explanation: String,
}

/// Troubleshooting guide
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TroubleshootingGuide {
    /// Issue description
    pub issue: String,
    /// Diagnosis of the issue
    pub diagnosis: String,
    /// Steps to resolve
    pub steps: Vec<String>,
    /// Final resolution
    pub resolution: String,
}
