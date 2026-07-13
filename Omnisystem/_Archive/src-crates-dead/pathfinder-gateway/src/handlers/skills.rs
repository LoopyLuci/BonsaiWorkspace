use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use crate::errors::GatewayError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub difficulty: String,
    pub prerequisites: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSkillRequest {
    pub name: String,
    pub difficulty: String,
    pub prerequisites: Vec<String>,
}

pub async fn list_skills(
    state: web::Data<AppState>,
) -> Result<HttpResponse, GatewayError> {
    let skills = vec![
        Skill {
            id: "skill_1".to_string(),
            name: "Algebra Fundamentals".to_string(),
            difficulty: "beginner".to_string(),
            prerequisites: vec![],
        },
        Skill {
            id: "skill_2".to_string(),
            name: "Geometry Basics".to_string(),
            difficulty: "beginner".to_string(),
            prerequisites: vec!["skill_1".to_string()],
        },
    ];

    Ok(HttpResponse::Ok().json(skills))
}

pub async fn get_skill(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, GatewayError> {
    let skill_id = path.into_inner();

    let skill = Skill {
        id: skill_id,
        name: "Algebra Fundamentals".to_string(),
        difficulty: "beginner".to_string(),
        prerequisites: vec![],
    };

    Ok(HttpResponse::Ok().json(skill))
}

pub async fn create_skill(
    state: web::Data<AppState>,
    req: web::Json<CreateSkillRequest>,
) -> Result<HttpResponse, GatewayError> {
    let skill = Skill {
        id: uuid::Uuid::new_v4().to_string(),
        name: req.name.clone(),
        difficulty: req.difficulty.clone(),
        prerequisites: req.prerequisites.clone(),
    };

    Ok(HttpResponse::Created().json(skill))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/skills")
            .route("", web::get().to(list_skills))
            .route("", web::post().to(create_skill))
            .route("/{id}", web::get().to(get_skill))
    );
}
