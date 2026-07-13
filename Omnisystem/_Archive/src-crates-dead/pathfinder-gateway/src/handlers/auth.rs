use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use crate::errors::GatewayError;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user_id: String,
    pub token: String,
    pub email: String,
}

pub async fn register(
    state: web::Data<AppState>,
    req: web::Json<RegisterRequest>,
) -> Result<HttpResponse, GatewayError> {
    if req.email.is_empty() || req.password.is_empty() {
        return Err(GatewayError::BadRequest("Missing fields".to_string()));
    }

    let user_id = uuid::Uuid::new_v4().to_string();
    let token = format!("token_{}", uuid::Uuid::new_v4());

    Ok(HttpResponse::Created().json(AuthResponse {
        user_id,
        token,
        email: req.email.clone(),
    }))
}

pub async fn login(
    state: web::Data<AppState>,
    req: web::Json<LoginRequest>,
) -> Result<HttpResponse, GatewayError> {
    if req.email.is_empty() || req.password.is_empty() {
        return Err(GatewayError::BadRequest("Missing credentials".to_string()));
    }

    let token = format!("token_{}", uuid::Uuid::new_v4());

    Ok(HttpResponse::Ok().json(AuthResponse {
        user_id: uuid::Uuid::new_v4().to_string(),
        token,
        email: req.email.clone(),
    }))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/auth")
            .route("/register", web::post().to(register))
            .route("/login", web::post().to(login))
    );
}
