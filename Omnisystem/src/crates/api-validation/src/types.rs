use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiRequest {
    pub request_id: String,
    pub endpoint: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub query_params: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationRule {
    pub field: String,
    pub rule_type: String,
    pub required: bool,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimitRule {
    pub rule_id: String,
    pub requests_per_second: u32,
    pub burst_size: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeaderValidation {
    pub required_headers: Vec<String>,
    pub allowed_headers: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentTypeRule {
    pub allowed_types: Vec<String>,
    pub max_body_size_bytes: u64,
}
