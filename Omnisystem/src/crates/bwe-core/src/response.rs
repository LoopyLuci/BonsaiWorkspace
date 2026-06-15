use bytes::Bytes;
use serde::Serialize;
use std::collections::HashMap;

/// HTTP response
#[derive(Debug, Clone)]
pub struct BweResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
}

impl BweResponse {
    pub fn new(status: u16, body: impl Into<Bytes>) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: body.into(),
        }
    }

    pub fn ok(body: impl Into<Bytes>) -> Self {
        Self::new(200, body)
    }

    pub fn json<T: Serialize>(status: u16, data: &T) -> Result<Self, serde_json::Error> {
        let body = serde_json::to_vec(data)?;
        let mut response = Self::new(status, body);
        response.headers.insert("Content-Type".to_string(), "application/json".to_string());
        Ok(response)
    }

    pub fn json_ok<T: Serialize>(data: &T) -> Result<Self, serde_json::Error> {
        Self::json(200, data)
    }

    pub fn not_found() -> Self {
        Self::new(404, "Not Found")
    }

    pub fn internal_error(msg: &str) -> Self {
        Self::new(500, msg)
    }

    pub fn bad_request(msg: &str) -> Self {
        Self::new(400, msg)
    }

    pub fn unauthorized() -> Self {
        Self::new(401, "Unauthorized")
    }

    pub fn forbidden() -> Self {
        Self::new(403, "Forbidden")
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn with_status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }
}
