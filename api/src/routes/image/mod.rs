mod upload;

use axum::{
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use db::{
    object_id::{BaseImageId, ProjectId, UploadProfileId},
    upload_profiles, Permission,
};
use diesel::prelude::*;
use http::StatusCode;
use serde::Deserialize;
use serde_json::json;

use pic_store_db as db;

use crate::{
    auth::UserInfo, get_object_by_field_query, get_object_query, shared_state::State, Error,
};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum UploadProfileOrShortId {
    Id(UploadProfileId),
    ShortId(String),
}

#[derive(Deserialize, Debug)]
struct NewBaseImageInput {
    filename: String,
    location: Option<String>,
    alt_text: Option<String>,
    upload_profile_id: Option<UploadProfileOrShortId>,
}

async fn new_base_image(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    Json(payload): Json<NewBaseImageInput>,
) -> Result<impl IntoResponse, Error> {
    let upload_profile = payload
        .upload_profile_id
        .or_else(|| {
            user.default_upload_profile_id
                .map(UploadProfileOrShortId::Id)
        })
        .ok_or(Error::NoUploadProfile)?;

    let conn = state.db.get().await?;

    let image_id = conn
        .interact(move |conn| {
            #[derive(Debug, Queryable, Selectable)]
            #[diesel(table_name = upload_profiles)]
            struct UploadProfileInfo {
                id: UploadProfileId,
                project_id: ProjectId,
            }

            let (profile, allowed) = match upload_profile {
                UploadProfileOrShortId::Id(id) => get_object_query!(
                    upload_profiles,
                    conn,
                    user,
                    UploadProfileInfo,
                    id,
                    Permission::ImageCreate
                ),
                UploadProfileOrShortId::ShortId(short_id) => get_object_by_field_query!(
                    upload_profiles,
                    conn,
                    user,
                    UploadProfileInfo,
                    short_id,
                    short_id,
                    Permission::ImageCreate
                ),
            }?;

            if !allowed {
                return Err(Error::MissingPermission(Permission::ImageCreate));
            }

            let new_image_id = BaseImageId::new();

            let new_image = db::base_images::NewBaseImage {
                id: new_image_id,
                user_id: user.user_id,
                team_id: user.team_id,
                project_id: profile.project_id,
                upload_profile_id: profile.id,
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
            "id": image_id,
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
