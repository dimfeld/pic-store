mod upload;

use axum::{
    extract::Query,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use db::{
    object_id::{BaseImageId, UploadProfileId},
    Permission,
};
use diesel::prelude::*;
use http::StatusCode;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use pic_store_db as db;
use pic_store_storage as storage;

use crate::{auth::UserInfo, shared_state::State, Error};

#[derive(Deserialize, Debug)]
struct NewBaseImageInput {
    filename: String,
    upload_profile_id: String,
    alt_text: Option<String>,
}

#[derive(Deserialize)]
struct UploadProfileIdQuery {
    upload_profile_id: Option<UploadProfileId>,
}

async fn new_base_image(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    Query(q): Query<UploadProfileIdQuery>,
    Json(payload): Json<NewBaseImageInput>,
) -> Result<impl IntoResponse, Error> {
    // Take either a JSON blob with metadata about the image to upload,
    // or a multipart form which may or may not contain the image data.

    let upload_profile = q
        .upload_profile_id
        .or(user.default_upload_profile_id)
        .ok_or(Error::NoUploadProfile)?;

    let conn = state.db.get().await?;
    let image_id = BaseImageId::new();

    let (profile, location, allowed) = conn
        .interact(move |conn| {
            db::upload_profiles::table
                .inner_join(
                    db::storage_locations::table.on(db::storage_locations::storage_location_id
                        .eq(db::upload_profiles::base_storage_location_id)),
                )
                .filter(db::upload_profiles::upload_profile_id.eq(upload_profile))
                .select((
                    db::upload_profiles::all_columns,
                    db::storage_locations::all_columns,
                    db::obj_allowed!(
                        user.team_id,
                        &user.roles,
                        db::upload_profiles::project_id,
                        Permission::ImageCreate
                    ),
                ))
                .first::<(
                    db::upload_profiles::UploadProfile,
                    db::storage_locations::StorageLocation,
                    bool,
                )>(conn)
        })
        .await??;

    if !allowed {
        return Err(Error::MissingPermission(Permission::ImageCreate));
    }

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
    // 1. If this was uploaded through a signed url, fetch the image and
    // figure out its format and dimensions. Also calculate the hash.
    // 2. Mark it finished
    // 3. Enqueue conversions
    todo!();
}

pub fn configure() -> Router {
    let routes = Router::new()
        .route("/", post(new_base_image))
        .route("/:image_id", get(get_base_image))
        .route("/:image_id", put(update_base_image_info))
        .route("/:image_id", delete(remove_base_image))
        .route("/:image_id/create_upload_url", post(get_upload_url))
        .route("/:image_id/upload", post(upload::upload_image))
        .route("/:image_id/complete", post(finished_upload));

    Router::new().nest("/images", routes)
}
