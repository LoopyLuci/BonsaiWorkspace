use axum::{Router, routing::get, routing::post};
use std::sync::Arc;
use crate::routes;
use crate::auth::AuthLayer;

pub struct AppState {
    pub model_registry: Arc<model_registry::registry::ModelRegistry>,
    pub inference_engine: Arc<inference::engine::InferenceEngine>,
}

pub async fn run(host: &str, port: u16, state: AppState) {
    let shared_state = Arc::new(state);
    let app = Router::new()
        .nest("/v1", routes::openai_routes())
        .nest("/api", routes::native_routes())
        .layer(AuthLayer)
        .layer(tower_http::cors::CorsLayer::permissive())
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(shared_state);
    let addr = format!("{}:{}", host, port);
    tracing::info!("API Gateway listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
