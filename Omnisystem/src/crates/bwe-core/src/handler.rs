use crate::{BweRequest, BweResponse, RequestContext, Result};
use async_trait::async_trait;
use std::sync::Arc;

/// Core trait for request handlers
#[async_trait]
pub trait Handler: Send + Sync {
    async fn handle(&self, req: BweRequest, ctx: &RequestContext) -> Result<BweResponse>;
}

/// Function-based handler wrapper
pub struct HandlerFn<F>
where
    F: Fn(BweRequest, Arc<RequestContext>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<BweResponse>> + Send>> + Send + Sync,
{
    f: F,
}

impl<F> HandlerFn<F>
where
    F: Fn(BweRequest, Arc<RequestContext>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<BweResponse>> + Send>> + Send + Sync,
{
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

#[async_trait]
impl<F> Handler for HandlerFn<F>
where
    F: Fn(BweRequest, Arc<RequestContext>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<BweResponse>> + Send>> + Send + Sync,
{
    async fn handle(&self, req: BweRequest, ctx: &RequestContext) -> Result<BweResponse> {
        (self.f)(req, Arc::new(ctx.clone())).await
    }
}

/// Simple closure-based handler
#[async_trait]
impl<F> Handler for F
where
    F: Fn(BweRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<BweResponse>> + Send>> + Send + Sync,
{
    async fn handle(&self, req: BweRequest, _ctx: &RequestContext) -> Result<BweResponse> {
        (self)(req).await
    }
}
