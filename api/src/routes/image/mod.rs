mod upload;

use axum::{
    extract::Path,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use db::{
    base_images,
    object_id::{BaseImageId, ProjectId, UploadProfileId},
    upload_profiles, BaseImageStatus, ImageFormat, Permission,
};
use diesel::prelude::*;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;

use pic_store_db as db;
use tracing::{event, Level};

use crate::{
    auth::{Authenticated, UserInfo},
    get_object, get_object_by_field_query, get_object_query,
    shared_state::State,
    Error, Result,
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
    Authenticated(user): Authenticated,
    Json(payload): Json<NewBaseImageInput>,
) -> Result<impl IntoResponse, Error> {
    event!(Level::INFO, ?user);
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
                // TODO sanitize file path for standard path exploits
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

async fn get_base_image(
    Extension(ref state): Extension<State>,
    Authenticated(user): Authenticated,
    Path(image_id): Path<BaseImageId>,
) -> Result<impl IntoResponse> {
    #[derive(Debug, Queryable, Selectable, Serialize)]
    #[diesel(table_name = base_images)]
    pub struct BaseImageResult {
        pub id: BaseImageId,
        pub project_id: ProjectId,
        pub hash: Option<String>,
        pub filename: String,
        pub location: String,
        pub width: i32,
        pub height: i32,
        pub format: Option<ImageFormat>,
        pub upload_profile_id: UploadProfileId,
        pub status: BaseImageStatus,
        pub alt_text: String,
        pub placeholder: Option<String>,

        pub updated: chrono::DateTime<chrono::Utc>,
    }

    // TODO Join on output images
    let (info, allowed) = get_object!(
        base_images,
        state,
        user,
        BaseImageResult,
        image_id,
        db::Permission::ProjectRead
    )
    .await?;

    if !allowed {
        return Err(Error::NotFound);
    }

    Ok((StatusCode::OK, Json(info)))
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
