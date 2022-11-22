use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use db::{
    object_id::{ProjectId, StorageLocationId},
    permissions::ProjectPermission,
    storage_locations::{self, NewStorageLocation, Provider},
    Permission,
};
use pic_store_db as db;
use serde_json::json;

use crate::{
    auth::{Authenticated, UserInfo},
    create_object, disable_object, get_object, list_project_and_global_objects,
    shared_state::State,
    write_object, Error,
};

#[derive(Deserialize)]
pub struct ProjectStorageLocationPath {
    project_id: ProjectId,
    storage_location_id: StorageLocationId,
}

#[derive(Debug, Deserialize)]
pub struct StorageLocationInput {
    pub name: String,
    pub provider: Provider,
    pub base_location: String,
    pub public_url_base: String,
}

#[derive(Debug, Serialize, Queryable, Selectable)]
#[diesel(table_name = db::storage_locations)]
pub struct StorageLocationOutput {
    pub id: StorageLocationId,
    pub name: String,
    pub provider: Provider,
    pub base_location: String,
    pub public_url_base: String,
    pub updated: DateTime<Utc>,
}

async fn list_global_locations(
    Extension(ref state): Extension<State>,
    Authenticated(user): Authenticated,
) -> Result<impl IntoResponse, Error> {
    list_locations(state, user, None).await
}

async fn list_project_locations(
    Extension(ref state): Extension<State>,
    Authenticated(user): Authenticated,
    Path(project_id): Path<ProjectId>,
) -> Result<impl IntoResponse, Error> {
    list_locations(state, user, Some(project_id)).await
}

async fn list_locations(
    state: &State,
    user: UserInfo,
    project_id: Option<ProjectId>,
) -> Result<impl IntoResponse, Error> {
    let objects = list_project_and_global_objects!(
        storage_locations,
        state,
        user,
        StorageLocationOutput,
        project_id,
        Permission::ProjectRead
    )
    .await?;

    Ok((StatusCode::OK, Json(objects)))
}

async fn write_project_location(
    Extension(ref state): Extension<State>,
    Authenticated(user): Authenticated,
    Path(path): Path<ProjectStorageLocationPath>,
    Json(body): Json<StorageLocationInput>,
) -> Result<impl IntoResponse, Error> {
    write_location(
        state,
        user,
        Some(path.project_id),
        path.storage_location_id,
        body,
    )
    .await
}

async fn write_global_location(
    Extension(ref state): Extension<State>,
    Authenticated(user): Authenticated,
    Path(location_id): Path<StorageLocationId>,
    Json(body): Json<StorageLocationInput>,
) -> Result<impl IntoResponse, Error> {
    write_location(state, user, None, location_id, body).await
}

async fn write_location(
    state: &State,
    user: UserInfo,
    project_id: Option<ProjectId>,
    location_id: StorageLocationId,
    body: StorageLocationInput,
) -> Result<impl IntoResponse, Error> {
    let result = write_object!(
        storage_locations,
        state,
        user,
        location_id,
        project_id.unwrap_or_else(ProjectId::nil),
        StorageLocationOutput,
        ProjectPermission::StorageLocationWrite,
        (
            dsl::name.eq(body.name),
            dsl::provider.eq(body.provider),
            dsl::base_location.eq(body.base_location),
            dsl::public_url_base.eq(body.public_url_base),
            dsl::updated.eq(Utc::now()),
        )
    )
    .await?;

    Ok((StatusCode::OK, Json(result)))
}

async fn new_project_location(
    Extension(ref state): Extension<State>,
    Authenticated(user): Authenticated,
    Path(project_id): Path<ProjectId>,
    Json(body): Json<StorageLocationInput>,
) -> Result<impl IntoResponse, crate::Error> {
    new_location(state, user, Some(project_id), body).await
}

async fn new_global_location(
    Extension(ref state): Extension<State>,
    Authenticated(user): Authenticated,
    Json(body): Json<StorageLocationInput>,
) -> Result<impl IntoResponse, crate::Error> {
    new_location(state, user, None, body).await
}

async fn new_location(
    state: &State,
    user: UserInfo,
    project_id: Option<ProjectId>,
    body: StorageLocationInput,
) -> Result<impl IntoResponse, Error> {
    let value = NewStorageLocation {
        id: StorageLocationId::new(),
        name: body.name,
        provider: body.provider,
        base_location: body.base_location,
        public_url_base: body.public_url_base,
        team_id: state.team_id,
        project_id,
    };

    let result = create_object!(
        storage_locations,
        state,
        user,
        project_id.unwrap_or_else(ProjectId::nil),
        StorageLocationOutput,
        ProjectPermission::StorageLocationWrite,
        &value
    )
    .await?;

    Ok((StatusCode::ACCEPTED, Json(result)))
}

async fn get_global_location(
    Extension(ref state): Extension<State>,
    Authenticated(user): Authenticated,
    Path(location_id): Path<StorageLocationId>,
) -> Result<impl IntoResponse, crate::Error> {
    get_location(state, user, location_id).await
}

async fn get_project_location(
    Extension(ref state): Extension<State>,
    Authenticated(user): Authenticated,
    Path(path): Path<ProjectStorageLocationPath>,
) -> Result<impl IntoResponse, crate::Error> {
    get_location(state, user, path.storage_location_id).await
}

async fn get_location(
    state: &State,
    user: UserInfo,
    location_id: StorageLocationId,
) -> Result<impl IntoResponse, crate::Error> {
    let (location, allowed) = get_object!(
        storage_locations,
        state,
        user,
        StorageLocationOutput,
        location_id,
        db::role_permissions::Permission::ProjectRead
    )
    .await?;

    if !allowed {
        return Err(Error::MissingPermission(
            db::role_permissions::Permission::ProjectRead,
        ));
    }

    Ok((StatusCode::OK, Json(location)))
}

async fn disable_project_location(
    Extension(ref state): Extension<State>,
    Authenticated(user): Authenticated,
    Path(path): Path<ProjectStorageLocationPath>,
) -> Result<impl IntoResponse, crate::Error> {
    disable_location(state, user, Some(path.project_id), path.storage_location_id).await
}

async fn disable_global_location(
    Extension(ref state): Extension<State>,
    Authenticated(user): Authenticated,
    Path(location_id): Path<StorageLocationId>,
) -> Result<impl IntoResponse, crate::Error> {
    disable_location(state, user, None, location_id).await
}

async fn disable_location(
    state: &State,
    user: UserInfo,
    project_id: Option<ProjectId>,
    location_id: StorageLocationId,
) -> Result<impl IntoResponse, crate::Error> {
    disable_object!(
        storage_locations,
        state,
        user,
        location_id,
        project_id.unwrap_or_else(ProjectId::nil),
        ProjectPermission::StorageLocationWrite
    )
    .await?;

    Ok((StatusCode::OK, Json(json!({}))))
}

pub fn configure() -> Router {
    let project_routes = Router::new()
        .route("/", get(list_project_locations))
        .route("/", post(new_project_location))
        .route("/:storage_location_id", get(get_project_location))
        .route("/:storage_location_id", put(write_project_location))
        .route("/:storage_location_id", delete(disable_project_location));

    let project_router =
        Router::new().nest("/projects/:project_id/storage_locations", project_routes);

    let global_routes = Router::new()
        .route("/", get(list_global_locations))
        .route("/", post(new_global_location))
        .route("/:storage_location_id", get(get_global_location))
        .route("/:storage_location_id", put(write_global_location))
        .route("/:storage_location_id", delete(disable_global_location));

    let global_router = Router::new().nest("/projects/global/storage_locations", global_routes);

    global_router.merge(project_router)
}
