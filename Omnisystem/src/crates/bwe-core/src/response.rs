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
        Self::new(500, msg.to_string())
    }

    pub fn bad_request(msg: &str) -> Self {
        Self::new(400, msg.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct Payload {
        ok: bool,
    }

    #[test]
    fn internal_error_carries_the_message() {
        let response = BweResponse::internal_error("boom");
        assert_eq!(response.status, 500);
        assert_eq!(&response.body[..], b"boom");
    }

    #[test]
    fn bad_request_carries_the_message() {
        let response = BweResponse::bad_request("nope");
        assert_eq!(response.status, 400);
        assert_eq!(&response.body[..], b"nope");
    }

    #[test]
    fn json_ok_sets_content_type() {
        let response = BweResponse::json_ok(&Payload { ok: true }).unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.headers.get("Content-Type"), Some(&"application/json".to_string()));
        assert_eq!(&response.body[..], br#"{"ok":true}"#);
    }

    #[test]
    fn with_header_and_with_status_are_chainable() {
        let response = BweResponse::ok("hi").with_status(201).with_header("X-Test", "1");
        assert_eq!(response.status, 201);
        assert_eq!(response.headers.get("X-Test"), Some(&"1".to_string()));
    }
}
