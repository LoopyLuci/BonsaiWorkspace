use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use crate::errors::GatewayError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub user_id: String,
    pub badge_id: String,
    pub rarity: String,
}

pub async fn list_achievements(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, GatewayError> {
    let user_id = path.into_inner();

    let achievements = vec![
        Achievement {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            badge_id: "badge_math_master".to_string(),
            rarity: "legendary".to_string(),
        },
    ];

    Ok(HttpResponse::Ok().json(achievements))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/achievements")
            .route("/user/{user_id}", web::get().to(list_achievements))
    );
}
