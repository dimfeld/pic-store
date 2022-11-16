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
    location: Option<String>,
    alt_text: Option<String>,
    upload_profile_id: Option<UploadProfileId>,
}

async fn new_base_image(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    Json(payload): Json<NewBaseImageInput>,
) -> Result<impl IntoResponse, Error> {
    // TODO Handle it might be a string
    let upload_profile = payload
        .upload_profile_id
        .or(user.default_upload_profile_id)
        .ok_or(Error::NoUploadProfile)?;

    let conn = state.db.get().await?;

    let image_id = conn
        .interact(move |conn| {
            let (profile, allowed) = db::upload_profiles::table
                .filter(db::upload_profiles::id.eq(upload_profile))
                .select((
                    db::upload_profiles::all_columns,
                    db::obj_allowed!(
                        user.team_id,
                        &user.roles,
                        db::upload_profiles::project_id,
                        Permission::ImageCreate
                    ),
                ))
                .first::<(db::upload_profiles::UploadProfile, bool)>(conn)?;

            if !allowed {
                return Err(Error::MissingPermission(Permission::ImageCreate));
            }

            let new_image_id = BaseImageId::new();

            let new_image = db::base_images::NewBaseImage {
                id: new_image_id,
                user_id: user.user_id,
                team_id: user.team_id,
                project_id: profile.project_id,
                upload_profile_id: upload_profile,
                filename: payload.filename.clone(),
                location: payload.location.unwrap_or(payload.filename),
                format: None,
                hash: String::new(),
                width: 0,
                height: 0,
                status: db::BaseImageStatus::AwaitingUpload,
                alt_text: payload.alt_text.unwrap_or_default(),
                placeholder: String::new(),
            };

            diesel::insert_into(db::base_images::table)
                .values(&new_image)
                .execute(conn)?;

            Ok(new_image_id)
        })
        .await??;

    Ok((
        StatusCode::OK,
        Json(json!({
            "image_id": image_id,
        })),
    ))
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

pub fn configure() -> Router {
    let routes = Router::new()
        .route("/", post(new_base_image))
        .route("/:image_id", get(get_base_image))
        .route("/:image_id", put(update_base_image_info))
        .route("/:image_id", delete(remove_base_image))
        .route("/:image_id/upload", post(upload::upload_image));

    Router::new().nest("/images", routes)
}
