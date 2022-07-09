use axum::{
    extract::{BodyStream, ContentLengthLimit, Multipart, Path},
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use db::object_id::{StorageLocationId, TeamId};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use pic_store_db as db;
use pic_store_storage as storage;

use crate::{shared_state::State, Error};

#[derive(Serialize)]
pub struct GetUploadUrlResponse {
    id: Uuid,
    url: Option<String>,
}

async fn get_upload_url(
    Path((profile_id, file_name)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    // TODO once it's built out this will fetch from the database
    let output_path = db::storage_locations::StorageLocation {
        storage_location_id: StorageLocationId::new(),
        team_id: TeamId::new(),
        project_id: None,
        name: "test storage location".to_string(),
        provider: db::storage_locations::Provider::Local,
        base_location: "./test_uploads".to_string(),
        public_url_base: "https://images.example.com".to_string(),
        updated: Utc::now(),
        deleted: None,
    };

    let provider = storage::Provider::from_db(output_path.provider)?;

    let destination = format!("{}/{}", output_path.base_location, file_name);

    let presigned_url = provider
        .create_operator(output_path.base_location.as_str())
        .await?
        .object(&file_name)
        .presign_write(time::Duration::hours(1))
        .map_err(Error::from_presign_error)?;

    // Add the entry to the database with some sort of pending tag
    // The client then uploads it to the backing store, and calls another endpoint (TBD) to mark it
    // done.

    let headers = presigned_url
        .header()
        .iter()
        .map(|(k, v)| (k.as_str(), v.to_str().unwrap_or_default()))
        .filter(|(_, v)| !v.is_empty())
        .collect::<Vec<_>>();

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({
            "method": presigned_url.method().as_str(),
            "uri": presigned_url.uri().to_string(),
            "headers": headers,
        })),
    ))
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
}
