use axum::{
    extract::{BodyStream, ContentLengthLimit, Multipart, Path},
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use blake2::Digest;
use bytes::Bytes;
use futures::{AsyncWriteExt, TryStreamExt};
use imageinfo::{ImageFormat, ImageInfo, ImageInfoError};
use sea_orm::{EntityTrait, Set};
use serde::Serialize;
use serde_json::json;
use time::OffsetDateTime;
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

    let provider = storage::Provider::from_db(
        output_path.provider,
        output_path
            .credentials
            .as_ref()
            .unwrap_or(&serde_json::Value::Null),
    )?;

    let destination = format!("{}/{}", output_path.base_location, file_name);

    let presigned_url = provider
        .create_presigned_upload_url(destination.as_str())
        .await?;

    // Add the entry to the database with some sort of pending tag
    // The client then uploads it to the backing store, and calls another endpoint (TBD) to mark it
    // done.

    let headers = presigned_url
        .headers
        .iter()
        .map(|(k, v)| (k.as_str(), v.to_str().unwrap_or_default()))
        .filter(|(_, v)| !v.is_empty())
        .collect::<Vec<_>>();

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({
            "method": presigned_url.method.as_str(),
            "uri": presigned_url.uri.to_string(),
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
