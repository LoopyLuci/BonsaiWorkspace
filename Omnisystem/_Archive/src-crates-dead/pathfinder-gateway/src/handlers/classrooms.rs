use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use crate::errors::GatewayError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Classroom {
    pub id: String,
    pub name: String,
    pub teacher_id: String,
    pub grade_level: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateClassroomRequest {
    pub name: String,
    pub grade_level: i32,
}

pub async fn list_classrooms(
    state: web::Data<AppState>,
) -> Result<HttpResponse, GatewayError> {
    let classrooms = vec![
        Classroom {
            id: "class_1".to_string(),
            name: "Algebra 101".to_string(),
            teacher_id: "teacher_1".to_string(),
            grade_level: 9,
        },
    ];

    Ok(HttpResponse::Ok().json(classrooms))
}

pub async fn create_classroom(
    state: web::Data<AppState>,
    req: web::Json<CreateClassroomRequest>,
) -> Result<HttpResponse, GatewayError> {
    let classroom = Classroom {
        id: uuid::Uuid::new_v4().to_string(),
        name: req.name.clone(),
        teacher_id: "teacher_1".to_string(),
        grade_level: req.grade_level,
    };

    Ok(HttpResponse::Created().json(classroom))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/classrooms")
            .route("", web::get().to(list_classrooms))
            .route("", web::post().to(create_classroom))
    );
}
