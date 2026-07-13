use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use crate::errors::GatewayError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Recommendation {
    pub skill_id: String,
    pub confidence: f64,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecommendationResponse {
    pub recommendations: Vec<Recommendation>,
}

pub async fn get_recommendations(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, GatewayError> {
    let user_id = path.into_inner();

    let recommendations = vec![
        Recommendation {
            skill_id: "skill_2".to_string(),
            confidence: 0.92,
            reason: "Based on your geometry progress".to_string(),
        },
        Recommendation {
            skill_id: "skill_3".to_string(),
            confidence: 0.87,
            reason: "Recommended for advanced learners".to_string(),
        },
    ];

    Ok(HttpResponse::Ok().json(RecommendationResponse { recommendations }))
}

pub async fn adjust_difficulty(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, GatewayError> {
    let (user_id, skill_id) = path.into_inner();

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "skill_id": skill_id,
        "new_difficulty": "advanced",
        "reason": "Based on 85% mastery"
    })))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/personalization")
            .route("/user/{user_id}/recommendations", web::get().to(get_recommendations))
            .route("/user/{user_id}/skill/{skill_id}/difficulty", web::post().to(adjust_difficulty))
    );
}
