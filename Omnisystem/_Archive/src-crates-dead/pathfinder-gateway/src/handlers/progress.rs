use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use crate::errors::GatewayError;

#[derive(Debug, Serialize, Deserialize)]
pub struct SkillProgress {
    pub user_id: String,
    pub skill_id: String,
    pub p_know: f64,
    pub attempts: i32,
}

pub async fn get_progress(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, GatewayError> {
    let (user_id, skill_id) = path.into_inner();

    let progress = SkillProgress {
        user_id,
        skill_id,
        p_know: 0.75,
        attempts: 15,
    };

    Ok(HttpResponse::Ok().json(progress))
}

pub async fn list_progress(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, GatewayError> {
    let user_id = path.into_inner();

    let progress = vec![
        SkillProgress {
            user_id: user_id.clone(),
            skill_id: "skill_1".to_string(),
            p_know: 0.85,
            attempts: 20,
        },
        SkillProgress {
            user_id: user_id.clone(),
            skill_id: "skill_2".to_string(),
            p_know: 0.65,
            attempts: 10,
        },
    ];

    Ok(HttpResponse::Ok().json(progress))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/progress")
            .route("/user/{user_id}", web::get().to(list_progress))
            .route("/user/{user_id}/skill/{skill_id}", web::get().to(get_progress))
    );
}
