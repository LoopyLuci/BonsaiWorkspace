use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use crate::errors::GatewayError;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub result_type: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub query: String,
}

pub async fn search(
    state: web::Data<AppState>,
    query: web::Query<SearchQuery>,
) -> Result<HttpResponse, GatewayError> {
    let results = vec![
        SearchResult {
            id: "skill_1".to_string(),
            title: "Algebra Fundamentals".to_string(),
            result_type: "skill".to_string(),
        },
    ];

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "query": query.query,
        "results": results,
        "total": results.len()
    })))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/search")
            .route("", web::get().to(search))
    );
}
