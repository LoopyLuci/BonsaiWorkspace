use actix_cors::Cors;
use actix_web::{web, App, HttpServer, middleware::Logger};
use pathfinder_core::database::Database;
use std::sync::Arc;
use tracing_subscriber;

mod handlers;
mod middleware;
mod errors;
mod state;

use handlers::*;
use state::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/pathfinder".to_string());

    let db = Database::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    db.run_migrations()
        .await
        .expect("Failed to initialize schema");

    let app_state = web::Data::new(AppState {
        db: Arc::new(db),
    });

    tracing::info!("Starting PATHFINDER Gateway on 0.0.0.0:8000");

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .app_data(app_state.clone())
            .wrap(cors)
            .wrap(Logger::default())
            .wrap(middleware::AuthMiddleware)
            .configure(auth::config)
            .configure(skills::config)
            .configure(exercises::config)
            .configure(progress::config)
            .configure(classrooms::config)
            .configure(notifications::config)
            .configure(achievements::config)
            .configure(search::config)
            .configure(personalization::config)
            .configure(analytics::config)
            .service(
                web::scope("/health")
                    .route("", web::get().to(health_check))
            )
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await
}

async fn health_check() -> actix_web::HttpResponse {
    actix_web::HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now()
    }))
}
