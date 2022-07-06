mod upload;

use axum::{
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use http::StatusCode;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use pic_store_auth::RequireBiscuit;
use pic_store_db as db;
use pic_store_storage as storage;

use crate::{shared_state::State, Error};

#[derive(Deserialize, Debug)]
struct NewBaseImageInput {
    filename: String,
    upload_profile_id: String,
    alt_text: Option<String>,
}

async fn new_base_image(
    Extension(ref state): Extension<State>,
    Json(payload): Json<NewBaseImageInput>,
    biscuit: RequireBiscuit,
) -> Result<impl IntoResponse, Error> {
    // Take either a JSON blob with metadata about the image to upload,
    // or a multipart form which may or may not contain the image data.
    let mut auth = state.auth.with_biscuit(&biscuit)?;
    let user_info = auth.get_user_and_team()?;

    // let a = db::upload_profiles::UploadProfile::find_by_id_or_short_id(
    //     user_info.team_id,
    //     payload.upload_profile_id.as_str(),
    // )
    // .one(&state.db)
    // .await?
    // .ok_or(Error::ObjectNotFound("upload_profile_id"))?;

    // let image_id = Uuid::new_v4();

    // let obj = db::base_image::ActiveModel {
    //     filename: Set(payload.filename.clone()),
    //     user_id: Set(user_info.user_id),
    //     team_id: Set(user_info.team_id),
    //     // TODO Use the project id linked to the upload profile
    //     // TODO verify that user has access to upload images to this project
    //     project_id: Set(state.project_id),
    //     // TODO verify that this profile exists and that we have access to it
    //     upload_profile_id: Set(profile.id),
    //     // TODO add a small random string to the end?
    //     location: Set(payload.filename),
    //     ..Default::default()
    // };

    // obj.insert(&state.db).await?;

    Ok((StatusCode::OK, Json(json!({}))))
}

async fn get_base_image() -> impl IntoResponse {
    todo!();
}

async fn remove_base_image() -> impl IntoResponse {
    // Set the base image to deleting, and queue jobs to delete
    // all the output images.
    todo!();
}

async fn update_base_image_info() -> impl IntoResponse {
    todo!();
}

async fn get_upload_url() -> impl IntoResponse {
    // Generate a new upload url, if applicable.
    todo!();
}

async fn finished_upload() -> impl IntoResponse {
    // 1. If this was uploaded through a signed url, fetch the first bit of the image and
    // figure out its format and dimensions
    // 2. Mark it finished
    // 3. Enqueue conversions
    todo!();
}

pub fn configure() -> Router {
    Router::new()
        .route("/", post(new_base_image))
        .route("/:image", get(get_base_image))
        .route("/:image", put(update_base_image_info))
        .route("/:image", delete(remove_base_image))
        .route("/:image_id/upload_url", post(get_upload_url))
        .route("/:image_id/upload", post(upload::upload_image))
        .route("/:image_id/finished_upload", post(finished_upload))
}
