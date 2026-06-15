//! Omnisystem Web Framework (OWF)
//!
//! Enterprise-grade HTTP server with zero external dependencies.
//! Supports HTTP/1.1, routing, middleware, WebSockets (phase 2).
//!
//! # Features
//!
//! - **Zero-copy HTTP parsing**: Efficient request handling
//! - **URL Routing**: Pattern-based request routing
//! - **Middleware Pipeline**: Composable request/response processing
//! - **Streaming Responses**: Memory-efficient large responses
//! - **TLS/SSL Support**: Secure connections
//! - **WebSocket Ready**: Foundation for WebSocket support

pub mod http;
pub mod server;
pub mod router;
pub mod middleware;

pub use http::{Request, Response, Method, StatusCode, Headers};
pub use server::{Server, HttpServer};
pub use router::{Router, RouteHandler};
pub use middleware::Middleware;

use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

/// Server statistics
#[derive(Debug, Clone)]
pub struct ServerStats {
    /// Total requests handled
    pub total_requests: usize,
    /// Active connections
    pub active_connections: usize,
    /// Total bytes sent
    pub bytes_sent: usize,
    /// Total bytes received
    pub bytes_received: usize,
    /// Request errors
    pub errors: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_method_parse() {
        assert_eq!(Method::from_str("GET"), Some(Method::Get));
        assert_eq!(Method::from_str("POST"), Some(Method::Post));
        assert_eq!(Method::from_str("PUT"), Some(Method::Put));
        assert_eq!(Method::from_str("DELETE"), Some(Method::Delete));
    }

    #[test]
    fn test_status_code() {
        assert_eq!(StatusCode::Ok.code(), 200);
        assert_eq!(StatusCode::NotFound.code(), 404);
        assert_eq!(StatusCode::InternalServerError.code(), 500);
    }
}
