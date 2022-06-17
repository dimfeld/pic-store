use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use serde::Serialize;
use uuid::Uuid;

async fn upload_image() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn list_profiles() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn write_profile() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn new_profile() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn get_profile(Path(profile_id): Path<Uuid>) -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED, profile_id.to_string())
}

async fn disable_profile() -> impl IntoResponse {
    todo!();
    StatusCode::NOT_IMPLEMENTED
}

pub fn configure() -> Router {
    let routes = Router::new()
        .route("/", get(list_profiles))
        .route("/", post(new_profile))
        .route("/:profile_id", get(get_profile))
        .route("/:profile_id", put(write_profile))
        .route("/:profile_id", delete(disable_profile))
        .route("/:profile_id/upload", post(upload_image));

    Router::new().nest("/profiles", routes)
}
