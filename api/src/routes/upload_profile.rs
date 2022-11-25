use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use chrono::Utc;
use db::{
    object_id::{ConversionProfileId, ProjectId, StorageLocationId, UploadProfileId},
    permissions::ProjectPermission,
    upload_profiles::{self, NewUploadProfile},
    Permission, PoolExt,
};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

use pic_store_db as db;

use crate::{
    auth::{must_have_permission_on_project, Authenticated, UserInfo},
    create_object, disable_object, get_object, list_project_objects,
    shared_state::AppState,
    write_object, Error, Result,
};

#[derive(Debug, Serialize, Deserialize)]
struct UploadProfileInput {
    pub name: String,
    pub short_id: Option<String>,
    pub base_storage_location_id: StorageLocationId,
    pub output_storage_location_id: StorageLocationId,
    pub conversion_profile_id: ConversionProfileId,
}

#[derive(Debug, Serialize, Queryable, Selectable)]
#[diesel(table_name = upload_profiles)]
struct UploadProfileOutput {
    pub name: String,
    pub short_id: Option<String>,
    pub base_storage_location_id: StorageLocationId,
    pub output_storage_location_id: StorageLocationId,
    pub conversion_profile_id: ConversionProfileId,
}

async fn list_project_upload_profiles(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path(project_id): Path<ProjectId>,
) -> Result<impl IntoResponse> {
    let objects = list_project_objects!(
        upload_profiles,
        state,
        user,
        UploadProfileOutput,
        project_id,
        ProjectPermission::UploadProfileRead
    )
    .await?;

    Ok((StatusCode::OK, Json(objects)))
}

async fn get_project_upload_profile(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path((project_id, profile_id)): Path<(ProjectId, UploadProfileId)>,
) -> Result<impl IntoResponse> {
    let (object, allowed) = get_object!(
        upload_profiles,
        state,
        user,
        UploadProfileOutput,
        profile_id,
        db::role_permissions::Permission::ProjectRead
    )
    .await?;

    if !allowed {
        return Err(Error::MissingPermission(
            db::role_permissions::Permission::ProjectRead,
        ));
    }

    Ok((StatusCode::OK, Json(object)))
}

async fn write_project_upload_profile(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path((project_id, profile_id)): Path<(ProjectId, UploadProfileId)>,
    Json(body): Json<UploadProfileInput>,
) -> Result<impl IntoResponse> {
    let result = write_object!(
        upload_profiles,
        state,
        user,
        profile_id,
        project_id,
        UploadProfileOutput,
        ProjectPermission::ProjectWrite,
        (dsl::name.eq(body.name), dsl::updated.eq(Utc::now()))
    )
    .await?;

    Ok((StatusCode::OK, Json(result)))
}

async fn new_project_upload_profile(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path(project_id): Path<ProjectId>,
    Json(payload): Json<UploadProfileInput>,
) -> Result<impl IntoResponse> {
    use db::upload_profiles::dsl;

    let value = NewUploadProfile {
        id: UploadProfileId::new(),
        name: payload.name,
        short_id: payload.short_id.unwrap_or_default(),
        base_storage_location_id: payload.base_storage_location_id,
        output_storage_location_id: payload.output_storage_location_id,
        conversion_profile_id: payload.conversion_profile_id,
        project_id,
        team_id: user.team_id,
    };

    let result = create_object!(
        upload_profiles,
        state,
        user,
        project_id,
        UploadProfileOutput,
        ProjectPermission::ProjectWrite,
        &value
    )
    .await?;

    Ok((StatusCode::OK, Json(result)))
}

async fn disable_project_upload_profile(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path((project_id, profile_id)): Path<(ProjectId, UploadProfileId)>,
) -> Result<impl IntoResponse> {
    disable_object!(
        upload_profiles,
        state,
        user,
        profile_id,
        project_id,
        ProjectPermission::ProjectWrite
    )
    .await?;

    Ok((StatusCode::OK, Json(json!({}))))
}

pub fn configure() -> Router<AppState> {
    let project_routes = Router::new()
        .route("/", get(list_project_upload_profiles))
        .route("/", post(new_project_upload_profile))
        .route("/:upload_profile_id", get(get_project_upload_profile))
        .route("/:upload_profile_id", put(write_project_upload_profile))
        .route(
            "/:upload_profile_id",
            delete(disable_project_upload_profile),
        );

    Router::new().nest("/projects/:project_id/upload_profiles", project_routes)
}
