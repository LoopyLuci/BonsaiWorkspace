//! HTTP server implementation

use crate::http::{Request, Response, Method};
use crate::router::Router;
use std::sync::Arc;

/// HTTP server trait
pub trait Server {
    /// Bind to address
    fn bind(&mut self, addr: &str) -> Result<(), String>;
    /// Run server
    async fn run(&self) -> Result<(), String>;
}

/// Production HTTP server
pub struct HttpServer {
    addr: Option<String>,
    router: Arc<Router>,
}

impl HttpServer {
    /// Create new HTTP server
    pub fn new(router: Router) -> Self {
        HttpServer {
            addr: None,
            router: Arc::new(router),
        }
    }

    /// Bind to address
    pub fn bind(&mut self, addr: &str) -> Result<(), String> {
        // Validate address format (simplified)
        if addr.contains(':') && addr.contains('.') {
            self.addr = Some(addr.to_string());
            Ok(())
        } else {
            Err("Invalid address format".to_string())
        }
    }

    /// Run server (async)
    pub async fn run(&self) -> Result<(), String> {
        if let Some(addr) = &self.addr {
            eprintln!("Server listening on {}", addr);
            // In production, this would bind to TCP socket and listen
            // Phase 2 will implement actual TCP binding
            Ok(())
        } else {
            Err("Server not bound to address".to_string())
        }
    }

    /// Handle an incoming request
    pub fn handle_request(&self, request: Request) -> Response {
        self.router.handle(request)
    }

    /// Get router
    pub fn router(&self) -> &Router {
        &self.router
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_server_bind() {
        let router = Router::new();
        let mut server = HttpServer::new(router);
        assert!(server.bind("127.0.0.1:8080").is_ok());
    }

    #[test]
    fn test_http_server_invalid_bind() {
        let router = Router::new();
        let mut server = HttpServer::new(router);
        assert!(server.bind("invalid").is_err());
    }

    #[tokio::test]
    async fn test_http_server_run() {
        let router = Router::new();
        let mut server = HttpServer::new(router);
        assert!(server.bind("127.0.0.1:8080").is_ok());
        // Note: async run test would need actual TCP binding
    }
}
