use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use crate::errors::GatewayError;

#[derive(Debug, Serialize, Deserialize)]
pub struct UserMetrics {
    pub total_attempts: i32,
    pub average_score: f64,
    pub current_streak: i32,
    pub time_spent_minutes: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassroomStats {
    pub total_students: i32,
    pub average_mastery: f64,
    pub completion_rate: f64,
    pub at_risk_count: i32,
}

pub async fn get_user_metrics(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, GatewayError> {
    let user_id = path.into_inner();

    let metrics = UserMetrics {
        total_attempts: 156,
        average_score: 0.82,
        current_streak: 7,
        time_spent_minutes: 420,
    };

    Ok(HttpResponse::Ok().json(metrics))
}

pub async fn get_classroom_stats(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, GatewayError> {
    let classroom_id = path.into_inner();

    let stats = ClassroomStats {
        total_students: 28,
        average_mastery: 0.72,
        completion_rate: 0.89,
        at_risk_count: 3,
    };

    Ok(HttpResponse::Ok().json(stats))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/analytics")
            .route("/user/{user_id}/metrics", web::get().to(get_user_metrics))
            .route("/classroom/{classroom_id}/stats", web::get().to(get_classroom_stats))
    );
}
