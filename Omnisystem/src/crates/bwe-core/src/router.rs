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
