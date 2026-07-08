use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

#[derive(Debug)]
pub enum GatewayError {
    NotFound(String),
    Unauthorized,
    BadRequest(String),
    InternalServer(String),
}

impl fmt::Display for GatewayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GatewayError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            GatewayError::Unauthorized => write!(f, "Unauthorized"),
            GatewayError::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            GatewayError::InternalServer(msg) => write!(f, "Internal Server Error: {}", msg),
        }
    }
}

impl ResponseError for GatewayError {
    fn error_response(&self) -> HttpResponse {
        match self {
            GatewayError::NotFound(msg) => {
                HttpResponse::NotFound().json(ApiError {
                    code: "NOT_FOUND".to_string(),
                    message: msg.clone(),
                })
            }
            GatewayError::Unauthorized => {
                HttpResponse::Unauthorized().json(ApiError {
                    code: "UNAUTHORIZED".to_string(),
                    message: "Authentication required".to_string(),
                })
            }
            GatewayError::BadRequest(msg) => {
                HttpResponse::BadRequest().json(ApiError {
                    code: "BAD_REQUEST".to_string(),
                    message: msg.clone(),
                })
            }
            GatewayError::InternalServer(msg) => {
                HttpResponse::InternalServerError().json(ApiError {
                    code: "INTERNAL_SERVER_ERROR".to_string(),
                    message: msg.clone(),
                })
            }
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            GatewayError::NotFound(_) => StatusCode::NOT_FOUND,
            GatewayError::Unauthorized => StatusCode::UNAUTHORIZED,
            GatewayError::BadRequest(_) => StatusCode::BAD_REQUEST,
            GatewayError::InternalServer(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
