//! HTTP routing

use crate::http::{Request, Response, Method};
use std::collections::HashMap;

/// Route handler type
pub type RouteHandler = fn(Request) -> Response;

/// HTTP router
pub struct Router {
    routes: HashMap<String, RouteHandler>,
    not_found_handler: Option<RouteHandler>,
}

impl Router {
    /// Create new router
    pub fn new() -> Self {
        Router {
            routes: HashMap::new(),
            not_found_handler: None,
        }
    }

    /// Register a GET route
    pub fn get(&mut self, path: impl Into<String>, handler: RouteHandler) {
        self.routes.insert(format!("GET:{}", path.into()), handler);
    }

    /// Register a POST route
    pub fn post(&mut self, path: impl Into<String>, handler: RouteHandler) {
        self.routes.insert(format!("POST:{}", path.into()), handler);
    }

    /// Register a PUT route
    pub fn put(&mut self, path: impl Into<String>, handler: RouteHandler) {
        self.routes.insert(format!("PUT:{}", path.into()), handler);
    }

    /// Register a DELETE route
    pub fn delete(&mut self, path: impl Into<String>, handler: RouteHandler) {
        self.routes.insert(format!("DELETE:{}", path.into()), handler);
    }

    /// Set 404 handler
    pub fn not_found(&mut self, handler: RouteHandler) {
        self.not_found_handler = Some(handler);
    }

    /// Handle a request
    pub fn handle(&self, request: Request) -> Response {
        let key = format!("{}:{}", request.method.as_str(), request.path);

        if let Some(handler) = self.routes.get(&key) {
            handler(request)
        } else if let Some(handler) = self.not_found_handler {
            handler(request)
        } else {
            Response::not_found()
        }
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_register_and_handle() {
        let mut router = Router::new();
        router.get("/api/users", |_req| {
            Response::ok().with_text("users")
        });

        let req = Request::new(Method::Get, "/api/users");
        let resp = router.handle(req);
        assert_eq!(resp.status, crate::http::StatusCode::Ok);
    }

    #[test]
    fn test_router_not_found() {
        let router = Router::new();
        let req = Request::new(Method::Get, "/unknown");
        let resp = router.handle(req);
        assert_eq!(resp.status, crate::http::StatusCode::NotFound);
    }

    #[test]
    fn test_router_multiple_methods() {
        let mut router = Router::new();
        router.get("/api/users", |_req| Response::ok().with_text("GET"));
        router.post("/api/users", |_req| Response::ok().with_text("POST"));

        let get_req = Request::new(Method::Get, "/api/users");
        let get_resp = router.handle(get_req);
        assert_eq!(get_resp.body, Some(b"GET".to_vec()));

        let post_req = Request::new(Method::Post, "/api/users");
        let post_resp = router.handle(post_req);
        assert_eq!(post_resp.body, Some(b"POST".to_vec()));
    }
}
