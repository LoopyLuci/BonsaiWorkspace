use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use uuid::Uuid;
use bytes::Bytes;

/// Unique identifier for a request
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RequestId(Uuid);

impl RequestId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
    Trace,
    Connect,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
            HttpMethod::Trace => "TRACE",
            HttpMethod::Connect => "CONNECT",
        }
    }
}

/// Incoming HTTP request
#[derive(Debug, Clone)]
pub struct BweRequest {
    pub id: RequestId,
    pub method: HttpMethod,
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
    pub remote_addr: String,
    pub timestamp: std::time::SystemTime,
}

impl BweRequest {
    pub fn new(
        method: HttpMethod,
        path: impl Into<String>,
        body: Bytes,
        remote_addr: impl Into<String>,
    ) -> Self {
        Self {
            id: RequestId::new(),
            method,
            path: path.into(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body,
            remote_addr: remote_addr.into(),
            timestamp: std::time::SystemTime::now(),
        }
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn with_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.insert(key.into(), value.into());
        self
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(|s| s.as_str())
    }

    pub fn query_param(&self, name: &str) -> Option<&str> {
        self.query.get(name).map(|s| s.as_str())
    }

    pub fn body_as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.body)
    }

    pub fn body_as_json<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_methods_set_headers_and_query() {
        let req = BweRequest::new(HttpMethod::Post, "/x", Bytes::from_static(b"{}"), "1.2.3.4".to_string())
            .with_header("X-Test", "value")
            .with_query("q", "rust");

        assert_eq!(req.header("X-Test"), Some("value"));
        assert_eq!(req.query_param("q"), Some("rust"));
        assert_eq!(req.header("missing"), None);
    }

    #[test]
    fn body_as_str_decodes_utf8() {
        let req = BweRequest::new(HttpMethod::Get, "/", Bytes::from_static(b"hello"), "127.0.0.1".to_string());
        assert_eq!(req.body_as_str().unwrap(), "hello");
    }

    #[test]
    fn body_as_json_deserializes() {
        #[derive(serde::Deserialize)]
        struct Payload {
            n: u32,
        }
        let req = BweRequest::new(HttpMethod::Post, "/", Bytes::from_static(b"{\"n\": 7}"), "127.0.0.1".to_string());
        let payload: Payload = req.body_as_json().unwrap();
        assert_eq!(payload.n, 7);
    }

    #[test]
    fn request_ids_are_unique() {
        let a = RequestId::new();
        let b = RequestId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn http_method_as_str_matches_standard_verbs() {
        assert_eq!(HttpMethod::Get.as_str(), "GET");
        assert_eq!(HttpMethod::Post.as_str(), "POST");
        assert_eq!(HttpMethod::Delete.as_str(), "DELETE");
    }
}
