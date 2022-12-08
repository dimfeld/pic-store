mod upload;

use axum::{
    extract::{DefaultBodyLimit, Path, State},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use db::{
    base_images,
    conversion_profiles::{ConversionFormat, ConversionSize},
    image_path,
    object_id::{BaseImageId, OutputImageId, ProjectId, UploadProfileId},
    output_images, projects, storage_locations, upload_profiles, BaseImageStatus, ImageFormat,
    OutputImageStatus, Permission, PoolExt,
};
use diesel::prelude::*;
use http::StatusCode;
use pic_store_db as db;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{event, Level};

use crate::{
    auth::{Authenticated, UserInfo},
    get_object_by_field_query, get_object_query,
    shared_state::AppState,
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
    State(state): State<AppState>,
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

            // TODO sanitize file path for standard path exploits
            // URL encoding is inconsistent between providers, so just replace any url-encoded
            // characters with a dash.
            static URLENCODED: once_cell::sync::OnceCell<regex::Regex> =
                once_cell::sync::OnceCell::new();
            let url_encoded =
                URLENCODED.get_or_init(|| regex::Regex::new(r##"[^a-zA-Z0-9-_.~]+"##).unwrap());
            let location = url_encoded
                .replace_all(payload.location.as_ref().unwrap_or(&payload.filename), "-")
                .to_string();

            let new_image = db::base_images::NewBaseImage {
                id: new_image_id,
                user_id: user.user_id,
                team_id: user.team_id,
                project_id: profile.project_id,
                upload_profile_id: profile.id,
                filename: payload.filename,
                location,
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

enum BaseImageFetchType {
    ById(BaseImageId),
    ByHash(String),
}

async fn get_base_image_by_hash(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path(hash): Path<String>,
) -> Result<impl IntoResponse> {
    get_base_image(state, user, BaseImageFetchType::ByHash(hash)).await
}

async fn get_base_image_by_id(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path(image_id): Path<BaseImageId>,
) -> Result<impl IntoResponse> {
    get_base_image(state, user, BaseImageFetchType::ById(image_id)).await
}

async fn get_base_image(
    state: AppState,
    user: UserInfo,
    lookup: BaseImageFetchType,
) -> Result<impl IntoResponse> {
    #[derive(Debug, Queryable, Selectable)]
    #[diesel(table_name = output_images)]
    struct OutputImageQueryResult {
        pub id: OutputImageId,
        pub location: String,
        pub file_size: i32,
        pub width: Option<i32>,
        pub height: Option<i32>,
        pub size: ConversionSize,
        pub format: ConversionFormat,

        pub status: OutputImageStatus,
        pub updated: chrono::DateTime<chrono::Utc>,
    }

    #[derive(Debug, Queryable, Selectable)]
    #[diesel(table_name = base_images)]
    struct BaseImageQueryResult {
        pub id: BaseImageId,
        pub project_id: ProjectId,
        pub hash: Option<String>,
        pub filename: String,
        pub file_size: i32,
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

    #[derive(Debug, Queryable, Selectable, Serialize)]
    #[diesel(table_name = storage_locations)]
    struct StorageLocationInfo {
        pub provider: db::storage_locations::Provider,
        pub base_location: String,
        pub public_url_base: String,
    }

    let (bst, ost) = diesel::alias!(storage_locations as bst, storage_locations as ost);
    let (
        info,
        base_storage,
        output_storage,
        project_base_path,
        profile_base_path,
        profile_output_path,
        output_images,
    ) = state
        .db
        .interact(move |conn| {
            let mut query = base_images::table
                .filter(base_images::deleted.is_null())
                .filter(base_images::team_id.eq(user.team_id))
                .inner_join(
                    db::upload_profiles::table
                        .on(base_images::upload_profile_id.eq(upload_profiles::id))
                        .inner_join(
                            bst.on(db::upload_profiles::base_storage_location_id
                                .eq(bst.field(db::storage_locations::id))),
                        )
                        .inner_join(
                            ost.on(db::upload_profiles::output_storage_location_id
                                .eq(ost.field(db::storage_locations::id))),
                        ),
                )
                .inner_join(db::projects::table.on(base_images::project_id.eq(db::projects::id)))
                .select((
                    BaseImageQueryResult::as_select(),
                    (
                        bst.field(storage_locations::provider),
                        bst.field(storage_locations::base_location),
                        bst.field(storage_locations::public_url_base),
                    ),
                    (
                        ost.field(storage_locations::provider),
                        ost.field(storage_locations::base_location),
                        ost.field(storage_locations::public_url_base),
                    ),
                    projects::base_location,
                    upload_profiles::base_storage_location_path,
                    upload_profiles::output_storage_location_path,
                    db::obj_allowed!(
                        user.team_id,
                        &user.roles,
                        base_images::project_id.assume_not_null(),
                        db::Permission::ProjectRead
                    ),
                ))
                .into_boxed();

            query = match lookup {
                BaseImageFetchType::ById(id) => query.filter(base_images::id.eq(id)),
                BaseImageFetchType::ByHash(hash) => query.filter(base_images::hash.eq(hash)),
            };

            let (
                info,
                base_storage,
                output_storage,
                project_base_path,
                profile_base_location,
                profile_output_location,
                allowed,
            ) = query
                .first::<(
                    BaseImageQueryResult,
                    StorageLocationInfo,
                    StorageLocationInfo,
                    String,
                    Option<String>,
                    Option<String>,
                    bool,
                )>(conn)
                .optional()
                .map_err(Error::from)?
                .ok_or(Error::NotFound)?;

            if !allowed {
                return Err(Error::NotFound);
            }

            let oi = output_images::table
                .filter(output_images::base_image_id.eq(info.id))
                .select(OutputImageQueryResult::as_select())
                .load::<OutputImageQueryResult>(conn)
                .map_err(Error::from)?;

            Ok((
                info,
                base_storage,
                output_storage,
                project_base_path,
                profile_base_location,
                profile_output_location,
                oi,
            ))
        })
        .await?;

    #[derive(Debug, Serialize)]
    struct OutputImageResult {
        pub id: OutputImageId,
        pub location: String,
        pub url: String,

        pub file_size: i32,
        pub width: Option<i32>,
        pub height: Option<i32>,
        pub size_rule: ConversionSize,
        pub format: ImageFormat,

        pub status: OutputImageStatus,

        pub updated: chrono::DateTime<chrono::Utc>,
    }

    #[derive(Debug, Serialize)]
    struct Result {
        pub id: BaseImageId,
        pub project_id: ProjectId,
        pub hash: Option<String>,
        pub filename: String,
        pub location: String,
        pub url: String,
        pub file_size: i32,
        pub width: i32,
        pub height: i32,
        pub format: Option<ImageFormat>,
        pub upload_profile_id: UploadProfileId,
        pub status: BaseImageStatus,
        pub alt_text: String,
        pub placeholder: Option<String>,

        pub updated: chrono::DateTime<chrono::Utc>,

        pub output: Vec<OutputImageResult>,
    }

    let base_image_path = image_path(
        &base_storage.base_location,
        &project_base_path,
        &profile_base_path,
        &info.location,
    );
    let base_image_url = image_path(
        &base_storage.public_url_base,
        &project_base_path,
        &profile_base_path,
        &info.location,
    );

    let output_images = output_images
        .into_iter()
        .map(|o| {
            let location = image_path(
                &output_storage.base_location,
                &project_base_path,
                &profile_output_path,
                &o.location,
            );
            let url = image_path(
                &output_storage.public_url_base,
                &project_base_path,
                &profile_output_path,
                &o.location,
            );

            OutputImageResult {
                id: o.id,
                location,
                url,
                file_size: o.file_size,
                width: o.width,
                height: o.height,
                size_rule: o.size,
                format: o.format.as_db_image_format(),
                status: o.status,
                updated: o.updated,
            }
        })
        .collect::<Vec<_>>();

    let result = Result {
        id: info.id,
        project_id: info.project_id,
        hash: info.hash,
        filename: info.filename,
        location: base_image_path,
        url: base_image_url,
        file_size: info.file_size,
        width: info.width,
        height: info.height,
        format: info.format,
        upload_profile_id: info.upload_profile_id,
        status: info.status,
        alt_text: info.alt_text,
        placeholder: info.placeholder,
        updated: info.updated,
        output: output_images,
    };

    Ok((StatusCode::OK, Json(result)))
}

async fn remove_base_image() -> impl IntoResponse {
    // Set the base image to deleting, and queue jobs to delete
    // all the output images.
    todo!();
}

async fn update_base_image_info() -> impl IntoResponse {
    todo!();
}

pub fn configure() -> Router<AppState> {
    let routes = Router::new()
        .route("/", post(new_base_image))
        .route("/:image_id", get(get_base_image_by_id))
        .route("/:image_id", put(update_base_image_info))
        .route("/:image_id", delete(remove_base_image));

    let upload_route = Router::new()
        .route("/:image_id/upload", post(upload::upload_image))
        .layer(DefaultBodyLimit::max(250 * 1048576));

    let image_id_routes = Router::new().nest("/images", routes.merge(upload_route));

    Router::new()
        .route("/image_by_hash/:hash", get(get_base_image_by_hash))
        .merge(image_id_routes)
}
