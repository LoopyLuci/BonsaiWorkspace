use bwe_core::{BweBuilder, BweConfig, BweRequest, BweResponse, Result};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = BweConfig {
        host: "0.0.0.0".to_string(),
        port: 8080,
        service_name: "hello-world".to_string(),
        ..Default::default()
    };

    let mut builder = BweBuilder::new(config);

    // Register a simple JSON endpoint
    builder = builder.with_handler("/api/hello", |req: BweRequest| {
        Box::pin(async move {
            let response = serde_json::json!({
                "message": "Hello from Bonsai Web Engine!",
                "method": req.method.as_str(),
                "path": req.path,
            });

            BweResponse::json_ok(&response)
                .map_err(|e| bwe_core::error::BweError::Custom(e.to_string()))
        })
    });

    // Register a health check endpoint
    builder = builder.with_handler("/health", |_req: BweRequest| {
        Box::pin(async move {
            let response = serde_json::json!({
                "status": "healthy",
                "service": "bwe-hello-world",
            });

            BweResponse::json_ok(&response)
                .map_err(|e| bwe_core::error::BweError::Custom(e.to_string()))
        })
    });

    let server = builder.build().await?;
    println!("Starting Bonsai Web Engine server on http://0.0.0.0:8080");
    server.start().await?;

    Ok(())
}
