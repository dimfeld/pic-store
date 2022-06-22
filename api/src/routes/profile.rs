use axum::{
    extract::{BodyStream, ContentLengthLimit, Multipart, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use pic_store_auth::RequireBiscuit;
use pic_store_db as db;

use crate::Error;

async fn upload_image(
    Path((profile_id, file_name)): Path<(String, String)>,
    ContentLengthLimit(mut stream): ContentLengthLimit<BodyStream, { 250 * 1048576 }>,
) -> impl IntoResponse {
    // TODO once it's built out this will fetch from the database
    let output_path = db::storage_location::Model {
        id: Uuid::new_v4(),
        project_id: Uuid::new_v4(),
        name: "test storage location".to_string(),
        provider: db::storage_location::Provider::Local,
        base_location: "./test_uploads".to_string(),
        credentials: None,
        public_url_base: "https://images.example.com".to_string(),
        updated: OffsetDateTime::now_utc(),
        deleted: None,
    };

    // Create the upload stream
    // Stream the request BodyStream to the destination stream
}

#[derive(Serialize)]
pub struct GetUploadUrlResponse {
    id: Uuid,
    url: Option<String>,
}

async fn get_upload_url(
    Path((profile_id, file_name)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    // TODO once it's built out this will fetch from the database
    let output_path = db::storage_location::Model {
        id: Uuid::new_v4(),
        project_id: Uuid::new_v4(),
        name: "test storage location".to_string(),
        provider: db::storage_location::Provider::Local,
        base_location: "./test_uploads".to_string(),
        credentials: None,
        public_url_base: "https://images.example.com".to_string(),
        updated: OffsetDateTime::now_utc(),
        deleted: None,
    };

    let response = match output_path.provider {
        db::storage_location::Provider::S3 => todo!("Generate S3 upload URL"),
        db::storage_location::Provider::Local => Err(Error::NoUploadUrlError(output_path.provider)),
    }?;

    // Add the entry to the database with some sort of pending tag
    // The client then uploads it to the backing store, and calls another endpoint (TBD) to mark it
    // done.

    Ok(response)
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
    Router::new()
        .route("/", get(list_profiles))
        .route("/", post(new_profile))
        .route("/:profile_id", get(get_profile))
        .route("/:profile_id", put(write_profile))
        .route("/:profile_id", delete(disable_profile))
        .route(
            "/:profile_id/get_upload_url/:file_name",
            post(get_upload_url),
        )
        .route("/:profile_id/upload/:file_name", post(upload_image))
}
