//! HTTP primitives (zero external dependencies)

use std::collections::HashMap;

/// HTTP methods
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
    Connect,
    Trace,
}

impl Method {
    /// Parse method from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(Method::Get),
            "POST" => Some(Method::Post),
            "PUT" => Some(Method::Put),
            "DELETE" => Some(Method::Delete),
            "PATCH" => Some(Method::Patch),
            "HEAD" => Some(Method::Head),
            "OPTIONS" => Some(Method::Options),
            "CONNECT" => Some(Method::Connect),
            "TRACE" => Some(Method::Trace),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Patch => "PATCH",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
            Method::Connect => "CONNECT",
            Method::Trace => "TRACE",
        }
    }
}

/// HTTP status codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCode {
    Ok = 200,
    Created = 201,
    Accepted = 202,
    NoContent = 204,
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    InternalServerError = 500,
    NotImplemented = 501,
    ServiceUnavailable = 503,
}

impl StatusCode {
    /// Get numeric code
    pub fn code(&self) -> u16 {
        *self as u16
    }

    /// Get reason phrase
    pub fn reason(&self) -> &'static str {
        match self {
            StatusCode::Ok => "OK",
            StatusCode::Created => "Created",
            StatusCode::Accepted => "Accepted",
            StatusCode::NoContent => "No Content",
            StatusCode::BadRequest => "Bad Request",
            StatusCode::Unauthorized => "Unauthorized",
            StatusCode::Forbidden => "Forbidden",
            StatusCode::NotFound => "Not Found",
            StatusCode::MethodNotAllowed => "Method Not Allowed",
            StatusCode::InternalServerError => "Internal Server Error",
            StatusCode::NotImplemented => "Not Implemented",
            StatusCode::ServiceUnavailable => "Service Unavailable",
        }
    }
}

/// HTTP headers
#[derive(Debug, Clone)]
pub struct Headers {
    headers: HashMap<String, String>,
}

impl Headers {
    /// Create new headers
    pub fn new() -> Self {
        Headers {
            headers: HashMap::new(),
        }
    }

    /// Insert a header
    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.headers.insert(name.into().to_lowercase(), value.into());
    }

    /// Get a header value
    pub fn get(&self, name: &str) -> Option<&str> {
        self.headers.get(&name.to_lowercase()).map(|s| s.as_str())
    }

    /// Check if header exists
    pub fn contains(&self, name: &str) -> bool {
        self.headers.contains_key(&name.to_lowercase())
    }
}

impl Default for Headers {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP request
#[derive(Debug, Clone)]
pub struct Request {
    pub method: Method,
    pub path: String,
    pub query: Option<String>,
    pub headers: Headers,
    pub body: Option<Vec<u8>>,
}

impl Request {
    /// Create new request
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Request {
            method,
            path: path.into(),
            query: None,
            headers: Headers::new(),
            body: None,
        }
    }

    /// Get query parameter
    pub fn query_param(&self, name: &str) -> Option<String> {
        self.query.as_ref().and_then(|q| {
            q.split('&')
                .find_map(|pair| {
                    let mut parts = pair.split('=');
                    if parts.next()? == name {
                        parts.next().map(|s| s.to_string())
                    } else {
                        None
                    }
                })
        })
    }
}

/// HTTP response
#[derive(Debug, Clone)]
pub struct Response {
    pub status: StatusCode,
    pub headers: Headers,
    pub body: Option<Vec<u8>>,
}

impl Response {
    /// Create new response
    pub fn new(status: StatusCode) -> Self {
        Response {
            status,
            headers: Headers::new(),
            body: None,
        }
    }

    /// Create OK response
    pub fn ok() -> Self {
        Self::new(StatusCode::Ok)
    }

    /// Create 404 response
    pub fn not_found() -> Self {
        Self::new(StatusCode::NotFound)
    }

    /// Create 500 response
    pub fn error() -> Self {
        Self::new(StatusCode::InternalServerError)
    }

    /// Set response body
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    /// Set response body from string
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.body = Some(text.into().into_bytes());
        self.headers.insert("Content-Type", "text/plain");
        self
    }

    /// Set response body from JSON
    pub fn with_json(mut self, json: impl Into<String>) -> Self {
        self.body = Some(json.into().into_bytes());
        self.headers.insert("Content-Type", "application/json");
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_roundtrip() {
        let method = Method::Post;
        assert_eq!(Method::from_str(method.as_str()), Some(method));
    }

    #[test]
    fn test_status_code() {
        assert_eq!(StatusCode::Ok.code(), 200);
        assert_eq!(StatusCode::Ok.reason(), "OK");
    }

    #[test]
    fn test_headers() {
        let mut headers = Headers::new();
        headers.insert("Content-Type", "application/json");
        assert!(headers.contains("content-type"));
        assert_eq!(headers.get("Content-Type"), Some("application/json"));
    }

    #[test]
    fn test_request() {
        let req = Request::new(Method::Get, "/api/users");
        assert_eq!(req.method, Method::Get);
        assert_eq!(req.path, "/api/users");
    }

    #[test]
    fn test_response() {
        let resp = Response::ok().with_text("Hello World");
        assert_eq!(resp.status, StatusCode::Ok);
        assert_eq!(resp.body, Some(b"Hello World".to_vec()));
    }
}
