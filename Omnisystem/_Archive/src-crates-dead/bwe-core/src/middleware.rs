use crate::{BweRequest, BweResponse, RequestContext, Result};
use async_trait::async_trait;
use std::sync::Arc;

/// Middleware trait for request/response processing
#[async_trait]
pub trait Middleware: Send + Sync {
    async fn process(
        &self,
        req: BweRequest,
        ctx: &RequestContext,
        next: Box<dyn Fn(BweRequest, &RequestContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<BweResponse>> + Send>> + Send + Sync>,
    ) -> Result<BweResponse>;
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

    pub async fn execute<F>(
        &self,
        req: BweRequest,
        ctx: &RequestContext,
        handler: F,
    ) -> Result<BweResponse>
    where
        F: Fn(BweRequest, &RequestContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<BweResponse>> + Send>> + Send + Sync + 'static,
    {
        if self.middlewares.is_empty() {
            return handler(req, ctx).await;
        }

        let mut index = 0;
        self.execute_middleware(req, ctx, &handler, &mut index).await
    }

    async fn execute_middleware<F>(
        &self,
        req: BweRequest,
        ctx: &RequestContext,
        handler: &F,
        index: &mut usize,
    ) -> Result<BweResponse>
    where
        F: Fn(BweRequest, &RequestContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<BweResponse>> + Send>> + Send + Sync + 'static,
    {
        if *index >= self.middlewares.len() {
            return handler(req, ctx).await;
        }

        let current_mw = self.middlewares[*index].clone();
        *index += 1;

        current_mw
            .process(
                req,
                ctx,
                Box::new(|r, c| Box::pin(handler(r, c))),
            )
            .await
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}
