use axum::{
    extract::{Path, State, Json},
    routing::{get, post, put, delete as delete_route},
    Router, http::StatusCode,
};
use uuid::Uuid;
use std::sync::Arc;
use crate::{types::*, Manager};

pub fn router(manager: Manager) -> Router {
    let manager = Arc::new(manager);
    Router::new()
        .route("/", post(create_item).get(list_items))
        .route("/:id", get(get_item).put(update_item).delete(delete_item))
        .with_state(manager)
}

async fn create_item(
    State(manager): State<Arc<Manager>>,
    Json(req): Json<CreateRequest>,
) -> (StatusCode, Json<Record>) {
    match manager.create(req) {
        Ok(record) => (StatusCode::CREATED, Json(record)),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(Record::new("system".to_string()))),
    }
}

async fn get_item(
    State(manager): State<Arc<Manager>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Record>, StatusCode> {
    manager
        .get(id)
        .ok()
        .flatten()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn update_item(
    State(manager): State<Arc<Manager>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateRequest>,
) -> Result<Json<Record>, StatusCode> {
    manager
        .update(id, req)
        .map(Json)
        .map_err(|_| StatusCode::NOT_FOUND)
}

async fn delete_item(
    State(manager): State<Arc<Manager>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    manager
        .delete(id)
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|_| StatusCode::NOT_FOUND)
}

async fn list_items(
    State(manager): State<Arc<Manager>>,
) -> Json<ListResponse> {
    let items = manager.list();
    let count = items.len();
    Json(ListResponse { items, count })
}
