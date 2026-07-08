// Mock objects for testing
use serde_json::{json, Value};

pub struct MockModuleRequest {
    pub request_id: String,
    pub operation: String,
    pub args: Value,
}

impl MockModuleRequest {
    pub fn new(operation: &str) -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            operation: operation.to_string(),
            args: json!({}),
        }
    }

    pub fn with_args(mut self, args: Value) -> Self {
        self.args = args;
        self
    }
}

pub struct MockResponse {
    pub request_id: String,
    pub status: String,
    pub data: Value,
}

impl MockResponse {
    pub fn success(data: Value) -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            status: "success".to_string(),
            data,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            status: "error".to_string(),
            data: json!({"error": msg}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_request_creation() {
        let req = MockModuleRequest::new("user:register");
        assert_eq!(req.operation, "user:register");
    }

    #[test]
    fn test_mock_response_creation() {
        let resp = MockResponse::success(json!({"id": "123"}));
        assert_eq!(resp.status, "success");
    }
}
