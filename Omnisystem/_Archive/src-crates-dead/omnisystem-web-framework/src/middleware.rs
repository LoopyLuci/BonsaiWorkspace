//! Middleware pipeline

use crate::http::{Request, Response};

/// Middleware trait for request/response processing
pub trait Middleware: Send + Sync {
    /// Process request before handler
    fn process_request(&self, request: &mut Request) -> Option<Response> {
        None
    }

    /// Process response after handler
    fn process_response(&self, response: &mut Response) {}
}

/// Built-in logging middleware
pub struct LoggingMiddleware;

impl Middleware for LoggingMiddleware {
    fn process_request(&self, request: &mut Request) -> Option<Response> {
        eprintln!("{} {}", request.method.as_str(), request.path);
        None
    }

    fn process_response(&self, response: &mut Response) {
        eprintln!("Response: {}", response.status.code());
    }
}

/// Built-in CORS middleware
pub struct CorsMiddleware {
    allow_origin: String,
}

impl CorsMiddleware {
    /// Create CORS middleware
    pub fn new(allow_origin: impl Into<String>) -> Self {
        CorsMiddleware {
            allow_origin: allow_origin.into(),
        }
    }
}

impl Middleware for CorsMiddleware {
    fn process_response(&self, response: &mut Response) {
        response.headers.insert("Access-Control-Allow-Origin", &self.allow_origin);
    }
}

/// Built-in compression middleware (placeholder for phase 2)
pub struct CompressionMiddleware;

impl Middleware for CompressionMiddleware {
    fn process_response(&self, response: &mut Response) {
        // Compression support in phase 2
        response.headers.insert("Content-Encoding", "gzip");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_middleware() {
        let middleware = LoggingMiddleware;
        let mut request = crate::http::Request::new(crate::http::Method::Get, "/test");
        assert_eq!(middleware.process_request(&mut request), None);
    }

    #[test]
    fn test_cors_middleware() {
        let middleware = CorsMiddleware::new("*");
        let mut response = Response::ok();
        middleware.process_response(&mut response);
        assert!(response.headers.contains("Access-Control-Allow-Origin"));
    }
}
