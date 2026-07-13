use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use crate::errors::GatewayError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub user_id: String,
    pub message: String,
    pub notification_type: String,
}

pub async fn list_notifications(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, GatewayError> {
    let user_id = path.into_inner();

    let notifications = vec![
        Notification {
            id: "notif_1".to_string(),
            user_id,
            message: "You unlocked a new achievement!".to_string(),
            notification_type: "achievement".to_string(),
        },
    ];

    Ok(HttpResponse::Ok().json(notifications))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/notifications")
            .route("/user/{user_id}", web::get().to(list_notifications))
    );
}
