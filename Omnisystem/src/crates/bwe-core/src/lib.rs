//! Bonsai Web Engine (BWE) core: a small async HTTP framework primitive set
//! (request/response types, router, middleware chain, capability-aware
//! request context, and a raw-socket HTTP/1.1 server).

pub mod context;
pub mod error;
pub mod handler;
pub mod middleware;
pub mod request;
pub mod response;
pub mod router;
pub mod server;

pub use context::{CapabilityToken, RequestContext};
pub use error::{BweError, Result};
pub use handler::{Handler, HandlerFn};
pub use middleware::{Middleware, MiddlewareChain, NextFn};
pub use request::{BweRequest, HttpMethod, RequestId};
pub use response::BweResponse;
pub use router::Router;
pub use server::BweServer;

use std::sync::Arc;

/// Server configuration.
#[derive(Debug, Clone)]
pub struct BweConfig {
    pub host: String,
    pub port: u16,
    pub service_name: String,
}

impl Default for BweConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            service_name: "bwe-service".to_string(),
        }
    }
}

/// Fluent builder for assembling a [`BweServer`] from a router, middleware
/// chain, and config.
pub struct BweBuilder {
    config: BweConfig,
    router: Router,
    middleware_chain: MiddlewareChain,
}

impl BweBuilder {
    pub fn new(config: BweConfig) -> Self {
        Self {
            config,
            router: Router::new(),
            middleware_chain: MiddlewareChain::new(),
        }
    }

    /// Register a handler for an exact path (or a `/*` prefix route).
    pub fn with_handler<H: Handler + 'static>(mut self, path: impl Into<String>, handler: H) -> Self {
        self.router.register(path.into(), Arc::new(handler));
        self
    }

    /// Register a handler for unmatched routes.
    pub fn with_not_found_handler<H: Handler + 'static>(mut self, handler: H) -> Self {
        self.router.set_not_found_handler(Arc::new(handler));
        self
    }

    /// Append a middleware to the chain (runs in registration order).
    pub fn with_middleware(mut self, middleware: Arc<dyn Middleware>) -> Self {
        self.middleware_chain.add(middleware);
        self
    }

    /// Finalize the builder into a runnable [`BweServer`].
    pub async fn build(self) -> Result<BweServer> {
        if self.config.service_name.trim().is_empty() {
            return Err(BweError::Custom("service_name must not be empty".to_string()));
        }
        Ok(BweServer::new(self.config, self.router, self.middleware_chain))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn builder_produces_a_server_with_valid_config() {
        let config = BweConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            service_name: "test-service".to_string(),
        };
        let builder = BweBuilder::new(config).with_handler(
            "/ping",
            |_req: BweRequest| -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<BweResponse>> + Send>> {
                Box::pin(async { Ok(BweResponse::ok("pong")) })
            },
        );

        assert!(builder.build().await.is_ok());
    }

    #[tokio::test]
    async fn builder_rejects_empty_service_name() {
        let config = BweConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            service_name: "  ".to_string(),
        };
        let builder = BweBuilder::new(config);
        assert!(builder.build().await.is_err());
    }

    #[test]
    fn default_config_has_sane_defaults() {
        let config = BweConfig::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "127.0.0.1");
        assert!(!config.service_name.is_empty());
    }
}
