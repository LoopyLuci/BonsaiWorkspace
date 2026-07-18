//! Bonsai Web Engine CLI: exercises the real router + middleware chain
//! in-process (no socket needed) to demonstrate request handling.

use async_trait::async_trait;
use bwe_core::{
    BweRequest, BweResponse, HttpMethod, Middleware, MiddlewareChain, NextFn, RequestContext, Result, Router,
};
use std::pin::Pin;
use std::sync::Arc;

struct LoggingMiddleware;

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn process(&self, req: BweRequest, ctx: &RequestContext, next: NextFn) -> Result<BweResponse> {
        println!("--> {} {} (trace_id={})", req.method.as_str(), req.path, ctx.trace_id);
        let response = next(req, ctx).await?;
        println!("<-- {}", response.status);
        Ok(response)
    }
}

fn health_handler(_req: BweRequest) -> Pin<Box<dyn std::future::Future<Output = Result<BweResponse>> + Send>> {
    Box::pin(async { BweResponse::json_ok(&serde_json::json!({"status": "healthy"})).map_err(Into::into) })
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut router = Router::new();
    router.register("/health".to_string(), Arc::new(health_handler));
    let router = Arc::new(router);

    let mut chain = MiddlewareChain::new();
    chain.add(Arc::new(LoggingMiddleware));

    let ctx = RequestContext::new("bwe-cli").with_user("demo-user");
    let req = BweRequest::new(HttpMethod::Get, "/health", Default::default(), "127.0.0.1:0".to_string());

    let response = chain
        .execute(req, &ctx, move |r, c| {
            let router = router.clone();
            let ctx_owned = c.clone();
            Box::pin(async move { router.route(r, &ctx_owned).await })
        })
        .await?;

    println!(
        "Final response: status={} body={}",
        response.status,
        String::from_utf8_lossy(&response.body)
    );

    Ok(())
}
