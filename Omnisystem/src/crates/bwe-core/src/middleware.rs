use crate::{BweRequest, BweResponse, RequestContext, Result};
use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type NextFn = Box<
    dyn Fn(BweRequest, &RequestContext) -> Pin<Box<dyn Future<Output = Result<BweResponse>> + Send>>
        + Send
        + Sync,
>;

/// Middleware trait for request/response processing
#[async_trait]
pub trait Middleware: Send + Sync {
    async fn process(&self, req: BweRequest, ctx: &RequestContext, next: NextFn) -> Result<BweResponse>;
}

/// Middleware chain for composing multiple middlewares
pub struct MiddlewareChain {
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl MiddlewareChain {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    pub fn add(&mut self, middleware: Arc<dyn Middleware>) {
        self.middlewares.push(middleware);
    }

    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }

    pub async fn execute<F>(&self, req: BweRequest, ctx: &RequestContext, handler: F) -> Result<BweResponse>
    where
        F: Fn(BweRequest, &RequestContext) -> Pin<Box<dyn Future<Output = Result<BweResponse>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        let middlewares = Arc::new(self.middlewares.clone());
        let handler = Arc::new(handler);
        Self::run(middlewares, 0, handler, req, ctx.clone()).await
    }

    /// Drives the chain from `index` onward. Each middleware's `next` closure
    /// recurses into this same function at `index + 1`, so every middleware
    /// in the chain actually gets a chance to run (previously the `next`
    /// closure always jumped straight to the final handler, silently
    /// skipping every middleware after the first).
    fn run<F>(
        middlewares: Arc<Vec<Arc<dyn Middleware>>>,
        index: usize,
        handler: Arc<F>,
        req: BweRequest,
        ctx: RequestContext,
    ) -> Pin<Box<dyn Future<Output = Result<BweResponse>> + Send>>
    where
        F: Fn(BweRequest, &RequestContext) -> Pin<Box<dyn Future<Output = Result<BweResponse>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        Box::pin(async move {
            if index >= middlewares.len() {
                return handler(req, &ctx).await;
            }

            let mw = middlewares[index].clone();
            let next_middlewares = middlewares.clone();
            let next_handler = handler.clone();

            let next: NextFn = Box::new(move |r: BweRequest, c: &RequestContext| {
                let middlewares = next_middlewares.clone();
                let handler = next_handler.clone();
                let owned_ctx = c.clone();
                Box::pin(async move { Self::run(middlewares, index + 1, handler, r, owned_ctx).await })
            });

            mw.process(req, &ctx, next).await
        })
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HttpMethod;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct RecordingMiddleware {
        id: usize,
        order: Arc<std::sync::Mutex<Vec<usize>>>,
    }

    #[async_trait]
    impl Middleware for RecordingMiddleware {
        async fn process(&self, req: BweRequest, ctx: &RequestContext, next: NextFn) -> Result<BweResponse> {
            self.order.lock().unwrap().push(self.id);
            next(req, ctx).await
        }
    }

    fn make_handler() -> impl Fn(BweRequest, &RequestContext) -> Pin<Box<dyn Future<Output = Result<BweResponse>> + Send>>
           + Send
           + Sync
           + 'static {
        |_req, _ctx| Box::pin(async { Ok(BweResponse::ok("done")) })
    }

    #[tokio::test]
    async fn all_middlewares_run_in_order() {
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut chain = MiddlewareChain::new();
        chain.add(Arc::new(RecordingMiddleware { id: 1, order: order.clone() }));
        chain.add(Arc::new(RecordingMiddleware { id: 2, order: order.clone() }));
        chain.add(Arc::new(RecordingMiddleware { id: 3, order: order.clone() }));

        let req = BweRequest::new(HttpMethod::Get, "/", Default::default(), "127.0.0.1".to_string());
        let ctx = RequestContext::new("test");

        let response = chain.execute(req, &ctx, make_handler()).await.unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(*order.lock().unwrap(), vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn empty_chain_calls_handler_directly() {
        let chain = MiddlewareChain::new();
        let req = BweRequest::new(HttpMethod::Get, "/", Default::default(), "127.0.0.1".to_string());
        let ctx = RequestContext::new("test");

        let response = chain.execute(req, &ctx, make_handler()).await.unwrap();
        assert_eq!(response.status, 200);
    }

    struct ShortCircuitMiddleware {
        counter: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Middleware for ShortCircuitMiddleware {
        async fn process(&self, _req: BweRequest, _ctx: &RequestContext, _next: NextFn) -> Result<BweResponse> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            Ok(BweResponse::unauthorized())
        }
    }

    #[tokio::test]
    async fn middleware_can_short_circuit_without_calling_next() {
        let counter = Arc::new(AtomicUsize::new(0));
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut chain = MiddlewareChain::new();
        chain.add(Arc::new(ShortCircuitMiddleware { counter: counter.clone() }));
        chain.add(Arc::new(RecordingMiddleware { id: 99, order: order.clone() }));

        let req = BweRequest::new(HttpMethod::Get, "/", Default::default(), "127.0.0.1".to_string());
        let ctx = RequestContext::new("test");

        let response = chain.execute(req, &ctx, make_handler()).await.unwrap();
        assert_eq!(response.status, 401);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert!(order.lock().unwrap().is_empty(), "second middleware must not run");
    }
}
