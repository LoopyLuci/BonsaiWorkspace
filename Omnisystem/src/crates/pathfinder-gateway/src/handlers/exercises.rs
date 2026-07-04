use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use crate::errors::GatewayError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Exercise {
    pub id: String,
    pub skill_id: String,
    pub title: String,
    pub problem_type: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateExerciseRequest {
    pub skill_id: String,
    pub title: String,
    pub problem_type: String,
}

#[derive(Debug, Deserialize)]
pub struct SubmitAttemptRequest {
    pub exercise_id: String,
    pub answer: String,
}

pub async fn list_exercises(
    state: web::Data<AppState>,
) -> Result<HttpResponse, GatewayError> {
    let exercises = vec![
        Exercise {
            id: "ex_1".to_string(),
            skill_id: "skill_1".to_string(),
            title: "Solve 2x + 3 = 7".to_string(),
            problem_type: "equation".to_string(),
        },
    ];

    Ok(HttpResponse::Ok().json(exercises))
}

pub async fn get_exercise(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, GatewayError> {
    let exercise_id = path.into_inner();

    let exercise = Exercise {
        id: exercise_id,
        skill_id: "skill_1".to_string(),
        title: "Solve 2x + 3 = 7".to_string(),
        problem_type: "equation".to_string(),
    };

    Ok(HttpResponse::Ok().json(exercise))
}

pub async fn create_exercise(
    state: web::Data<AppState>,
    req: web::Json<CreateExerciseRequest>,
) -> Result<HttpResponse, GatewayError> {
    let exercise = Exercise {
        id: uuid::Uuid::new_v4().to_string(),
        skill_id: req.skill_id.clone(),
        title: req.title.clone(),
        problem_type: req.problem_type.clone(),
    };

    Ok(HttpResponse::Created().json(exercise))
}

pub async fn submit_attempt(
    state: web::Data<AppState>,
    req: web::Json<SubmitAttemptRequest>,
) -> Result<HttpResponse, GatewayError> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "exercise_id": req.exercise_id,
        "correct": true,
        "feedback": "Correct answer!".to_string()
    })))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/exercises")
            .route("", web::get().to(list_exercises))
            .route("", web::post().to(create_exercise))
            .route("/{id}", web::get().to(get_exercise))
            .route("/attempts", web::post().to(submit_attempt))
    );
}
