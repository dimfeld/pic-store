use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

use db::{
    conversion_profiles,
    conversion_profiles::{ConversionOutput, ConversionProfile, NewConversionProfile},
    object_id::{ConversionProfileId, ProjectId},
    permissions::ProjectPermission,
    Permission,
};
use pic_store_db as db;

use crate::{
    auth::{Authenticated, UserInfo},
    create_object, disable_object, get_object, list_project_and_global_objects,
    shared_state::AppState,
    write_object, Error,
};

#[derive(Debug, Deserialize)]
pub struct ConversionProfileInput {
    pub name: String,
    pub output: ConversionOutput,
}

#[derive(Debug, Serialize, Queryable, Selectable)]
#[diesel(table_name = conversion_profiles)]
pub struct ConversionProfileOutput {
    id: ConversionProfileId,
    name: String,
    output: ConversionOutput,
    updated: DateTime<Utc>,
}

impl From<ConversionProfile> for ConversionProfileOutput {
    fn from(value: ConversionProfile) -> Self {
        ConversionProfileOutput {
            id: value.id,
            name: value.name,
            output: value.output,
            updated: value.updated,
        }
    }
}

#[derive(Deserialize)]
pub struct ProjectConversionProfilePath {
    project_id: ProjectId,
    conversion_profile_id: ConversionProfileId,
}

/// List conversion profiles for the project and also the global projects.
async fn list_project_profiles(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path(project_id): Path<ProjectId>,
) -> Result<impl IntoResponse, crate::Error> {
    list_profiles(state, user, Some(project_id)).await
}

async fn list_global_profiles(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
) -> Result<impl IntoResponse, Error> {
    list_profiles(state, user, None).await
}

async fn list_profiles(
    state: AppState,
    user: UserInfo,
    project_id: Option<ProjectId>,
) -> Result<impl IntoResponse, Error> {
    let objects = list_project_and_global_objects!(
        conversion_profiles,
        state,
        user,
        ConversionProfileOutput,
        project_id,
        Permission::ProjectRead
    )
    .await?;

    Ok((StatusCode::OK, Json(objects)))
}

async fn write_project_profile(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path(path): Path<ProjectConversionProfilePath>,
    Json(body): Json<ConversionProfileInput>,
) -> Result<impl IntoResponse, Error> {
    write_profile(
        state,
        user,
        Some(path.project_id),
        path.conversion_profile_id,
        body,
    )
    .await
}

async fn write_global_profile(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path(profile_id): Path<ConversionProfileId>,
    Json(body): Json<ConversionProfileInput>,
) -> Result<impl IntoResponse, Error> {
    write_profile(state, user, None, profile_id, body).await
}

async fn write_profile(
    state: AppState,
    user: UserInfo,
    project_id: Option<ProjectId>,
    profile_id: ConversionProfileId,
    body: ConversionProfileInput,
) -> Result<impl IntoResponse, Error> {
    let result = write_object!(
        conversion_profiles,
        state,
        user,
        profile_id,
        project_id.unwrap_or_else(ProjectId::nil),
        ConversionProfileOutput,
        ProjectPermission::ConversionProfileWrite,
        (
            dsl::name.eq(body.name),
            dsl::output.eq(body.output),
            dsl::updated.eq(Utc::now())
        )
    )
    .await?;

    Ok((StatusCode::OK, Json(result)))
}

async fn new_project_profile(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path(project_id): Path<ProjectId>,
    Json(body): Json<ConversionProfileInput>,
) -> Result<impl IntoResponse, crate::Error> {
    new_profile(state, user, Some(project_id), body).await
}

async fn new_global_profile(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Json(body): Json<ConversionProfileInput>,
) -> Result<impl IntoResponse, crate::Error> {
    new_profile(state, user, None, body).await
}

async fn new_profile(
    state: AppState,
    user: UserInfo,
    project_id: Option<ProjectId>,
    body: ConversionProfileInput,
) -> Result<impl IntoResponse, Error> {
    let value = NewConversionProfile {
        id: ConversionProfileId::new(),
        name: body.name,
        team_id: state.team_id,
        project_id,
        output: body.output,
    };

    let result = create_object!(
        conversion_profiles,
        state,
        user,
        project_id.unwrap_or_else(ProjectId::nil),
        ConversionProfileOutput,
        ProjectPermission::ConversionProfileWrite,
        &value
    )
    .await?;

    Ok((StatusCode::ACCEPTED, Json(result)))
}

async fn get_global_profile(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path(profile_id): Path<ConversionProfileId>,
) -> Result<impl IntoResponse, crate::Error> {
    get_profile(state, user, profile_id).await
}

async fn get_project_profile(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path(path): Path<ProjectConversionProfilePath>,
) -> Result<impl IntoResponse, crate::Error> {
    get_profile(state, user, path.conversion_profile_id).await
}

async fn get_profile(
    state: AppState,
    user: UserInfo,
    profile_id: ConversionProfileId,
) -> Result<impl IntoResponse, crate::Error> {
    let (profile, allowed) = get_object!(
        conversion_profiles,
        state,
        user,
        ConversionProfileOutput,
        profile_id,
        db::role_permissions::Permission::ProjectRead
    )
    .await?;

    if !allowed {
        return Err(Error::MissingPermission(
            db::role_permissions::Permission::ProjectRead,
        ));
    }

    Ok((StatusCode::OK, Json(profile)))
}

async fn disable_project_profile(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path(path): Path<ProjectConversionProfilePath>,
) -> Result<impl IntoResponse, crate::Error> {
    disable_profile(
        state,
        user,
        Some(path.project_id),
        path.conversion_profile_id,
    )
    .await
}

async fn disable_global_profile(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path(profile_id): Path<ConversionProfileId>,
) -> Result<impl IntoResponse, crate::Error> {
    disable_profile(state, user, None, profile_id).await
}

async fn disable_profile(
    state: AppState,
    user: UserInfo,
    project_id: Option<ProjectId>,
    profile_id: ConversionProfileId,
) -> Result<impl IntoResponse, crate::Error> {
    disable_object!(
        conversion_profiles,
        state,
        user,
        profile_id,
        project_id.unwrap_or_else(ProjectId::nil),
        ProjectPermission::ConversionProfileWrite
    )
    .await?;

    Ok((StatusCode::OK, Json(json!({}))))
}

pub fn configure() -> Router<AppState> {
    let project_routes = Router::new()
        .route("/", get(list_project_profiles))
        .route("/", post(new_project_profile))
        .route("/:conversion_profile_id", get(get_project_profile))
        .route("/:conversion_profile_id", put(write_project_profile))
        .route("/:conversion_profile_id", delete(disable_project_profile));

    let project_router =
        Router::new().nest("/projects/:project_id/conversion_profiles", project_routes);

    let global_routes = Router::new()
        .route("/", get(list_global_profiles))
        .route("/", post(new_global_profile))
        .route("/:conversion_profile_id", get(get_global_profile))
        .route("/:conversion_profile_id", put(write_global_profile))
        .route("/:conversion_profile_id", delete(disable_global_profile));

    let global_router = Router::new().nest("/projects/global/conversion_profiles", global_routes);

    global_router.merge(project_router)
}
