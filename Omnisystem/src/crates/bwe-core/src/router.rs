use crate::{BweRequest, BweResponse, Handler, Result, RequestContext};
use std::collections::HashMap;
use std::sync::Arc;

/// Route registry mapping paths to handlers
pub struct Router {
    routes: HashMap<String, Arc<dyn Handler>>,
    not_found_handler: Option<Arc<dyn Handler>>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
            not_found_handler: None,
        }
    }

    pub fn register(&mut self, path: String, handler: Arc<dyn Handler>) {
        self.routes.insert(path, handler);
    }

    pub fn set_not_found_handler(&mut self, handler: Arc<dyn Handler>) {
        self.not_found_handler = Some(handler);
    }

    pub async fn route(&self, req: BweRequest, ctx: &RequestContext) -> Result<BweResponse> {
        let path = req.path.clone();

        // Exact match first
        if let Some(handler) = self.routes.get(&path) {
            return handler.handle(req, ctx).await;
        }

        // Try prefix match (simple glob matching)
        for (route, handler) in &self.routes {
            if route.ends_with("/*") {
                let prefix = &route[..route.len() - 2];
                if path.starts_with(prefix) {
                    return handler.handle(req, ctx).await;
                }
            }
        }

        // Use 404 handler or return default 404
        if let Some(handler) = &self.not_found_handler {
            handler.handle(req, ctx).await
        } else {
            Ok(BweResponse::not_found())
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
    use crate::HttpMethod;

    fn text_handler(body: &'static str) -> Arc<dyn Handler> {
        Arc::new(move |_req: BweRequest| -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<BweResponse>> + Send>> {
            Box::pin(async move { Ok(BweResponse::ok(body)) })
        })
    }

    fn get(path: &str) -> BweRequest {
        BweRequest::new(HttpMethod::Get, path, Default::default(), "127.0.0.1".to_string())
    }

    #[tokio::test]
    async fn exact_match_wins() {
        let mut router = Router::new();
        router.register("/hello".to_string(), text_handler("hi"));
        let ctx = RequestContext::new("test");

        let response = router.route(get("/hello"), &ctx).await.unwrap();
        assert_eq!(&response.body[..], b"hi");
    }

    #[tokio::test]
    async fn prefix_match_for_wildcard_routes() {
        let mut router = Router::new();
        router.register("/api/*".to_string(), text_handler("api"));
        let ctx = RequestContext::new("test");

        let response = router.route(get("/api/users/42"), &ctx).await.unwrap();
        assert_eq!(&response.body[..], b"api");
    }

    #[tokio::test]
    async fn unmatched_path_uses_default_404() {
        let router = Router::new();
        let ctx = RequestContext::new("test");

        let response = router.route(get("/nope"), &ctx).await.unwrap();
        assert_eq!(response.status, 404);
    }

    #[tokio::test]
    async fn unmatched_path_uses_custom_not_found_handler() {
        let mut router = Router::new();
        router.set_not_found_handler(text_handler("custom 404"));
        let ctx = RequestContext::new("test");

        let response = router.route(get("/nope"), &ctx).await.unwrap();
        assert_eq!(&response.body[..], b"custom 404");
    }

    #[tokio::test]
    async fn exact_match_takes_priority_over_wildcard() {
        let mut router = Router::new();
        router.register("/api/*".to_string(), text_handler("wildcard"));
        router.register("/api/users".to_string(), text_handler("exact"));
        let ctx = RequestContext::new("test");

        let response = router.route(get("/api/users"), &ctx).await.unwrap();
        assert_eq!(&response.body[..], b"exact");
    }
}
