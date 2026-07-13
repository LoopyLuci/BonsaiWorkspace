use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, delete, put},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing_subscriber;

mod models;
mod db;
mod handlers;
mod error;
mod auth;
mod middleware;

use error::AppError;

pub struct AppState {
    db: sqlx::PgPool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Load environment variables
    dotenv::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://user:password@localhost/app_manager".to_string());

    // Create database pool
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./src/db/migrations")
        .run(&pool)
        .await?;

    let state = Arc::new(AppState { db: pool });

    // Build router
    let app = Router::new()
        // Auth routes
        .route("/api/auth/register", post(handlers::auth::register))
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/api/auth/logout", post(handlers::auth::logout))
        .route("/api/auth/refresh", post(handlers::auth::refresh_token))

        // User routes
        .route("/api/users/me", get(handlers::users::get_profile))
        .route("/api/users/me", put(handlers::users::update_profile))
        .route("/api/users/me", delete(handlers::users::delete_account))

        // Device routes
        .route("/api/devices", get(handlers::devices::list_devices))
        .route("/api/devices", post(handlers::devices::create_device))
        .route("/api/devices/:device_id", delete(handlers::devices::remove_device))

        // Sync routes
        .route("/api/sync/push", post(handlers::sync::push_changes))
        .route("/api/sync/pull", get(handlers::sync::pull_changes))
        .route("/api/sync/status", get(handlers::sync::sync_status))
        .route("/api/sync/conflicts/:conflict_id", post(handlers::sync::resolve_conflict))

        // Favorite routes
        .route("/api/favorites", get(handlers::favorites::list_favorites))
        .route("/api/favorites", post(handlers::favorites::add_favorite))
        .route("/api/favorites/:app_id", delete(handlers::favorites::remove_favorite))

        // Settings routes
        .route("/api/settings", get(handlers::settings::get_settings))
        .route("/api/settings", put(handlers::settings::update_settings))

        // Review routes
        .route("/api/reviews/:app_id", get(handlers::reviews::get_reviews))
        .route("/api/reviews", post(handlers::reviews::create_review))

        // Installation routes
        .route("/api/installations", get(handlers::installations::list_installations))
        .route("/api/installations", post(handlers::installations::install_app))
        .route("/api/installations/:app_id", delete(handlers::installations::uninstall_app))

        // Health check
        .route("/api/health", get(health_check))

        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await?;

    tracing::info!("Server listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> Response {
    (
        StatusCode::OK,
        serde_json::json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }).to_string(),
    ).into_response()
}
