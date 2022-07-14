use axum::{
    extract::{Path, Query},
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
    storage_locations::{NewStorageLocation, Provider},
    Permission,
};
use pic_store_db as db;
use serde_json::json;

use crate::{
    auth::{must_have_permission_on_project, UserInfo},
    shared_state::State,
    Error,
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
    #[serde(rename = "id")]
    pub storage_location_id: StorageLocationId,
    pub name: String,
    pub provider: Provider,
    pub base_location: String,
    pub public_url_base: String,
    pub updated: DateTime<Utc>,
}

async fn list_global_locations(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
) -> Result<impl IntoResponse, Error> {
    list_locations(state, user, None).await
}

async fn list_project_locations(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    Path(project_id): Path<ProjectId>,
) -> Result<impl IntoResponse, Error> {
    list_locations(state, user, Some(project_id)).await
}

async fn list_locations(
    state: &State,
    user: UserInfo,
    project_id: Option<ProjectId>,
) -> Result<impl IntoResponse, Error> {
    use db::storage_locations::dsl;
    let conn = state.db.get().await?;

    let objects = conn
        .interact(move |conn| {
            let q = dsl::storage_locations
                .select(StorageLocationOutput::as_select())
                .into_boxed()
                .filter(db::obj_allowed_or_projectless!(
                    user.team_id,
                    &user.roles,
                    dsl::project_id,
                    Permission::ProjectRead
                ));

            let q = if let Some(project_id) = project_id {
                q.filter(
                    dsl::project_id
                        .is_null()
                        .or(dsl::project_id.is_not_distinct_from(project_id)),
                )
            } else {
                q.filter(dsl::project_id.is_null())
            };

            q.load::<StorageLocationOutput>(conn)
        })
        .await??;

    Ok((StatusCode::OK, Json(objects)))
}

async fn write_project_location(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
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
    Extension(user): Extension<UserInfo>,
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
    use db::storage_locations::dsl;

    let conn = state.db.get().await?;

    let result = conn
        .interact(move |conn| {
            must_have_permission_on_project(
                conn,
                &user,
                project_id.unwrap_or_else(ProjectId::nil),
                ProjectPermission::StorageLocationWrite,
            )?;

            let result = diesel::update(dsl::storage_locations)
                .filter(dsl::storage_location_id.eq(location_id))
                .filter(dsl::project_id.is_not_distinct_from(project_id))
                .filter(dsl::team_id.eq(user.team_id))
                .set((
                    dsl::name.eq(body.name),
                    dsl::provider.eq(body.provider),
                    dsl::base_location.eq(body.base_location),
                    dsl::public_url_base.eq(body.public_url_base),
                    dsl::updated.eq(Utc::now()),
                ))
                .returning(StorageLocationOutput::as_select())
                .get_result::<StorageLocationOutput>(conn)?;

            Ok::<_, Error>(result)
        })
        .await??;

    Ok((StatusCode::OK, Json(result)))
}

async fn new_project_location(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    Path(project_id): Path<ProjectId>,
    Json(body): Json<StorageLocationInput>,
) -> Result<impl IntoResponse, crate::Error> {
    new_location(state, user, Some(project_id), body).await
}

async fn new_global_location(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
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
    use db::storage_locations::dsl;

    let value = NewStorageLocation {
        storage_location_id: StorageLocationId::new(),
        name: body.name,
        provider: body.provider,
        base_location: body.base_location,
        public_url_base: body.public_url_base,
        team_id: state.team_id,
        project_id,
    };

    let conn = state.db.get().await?;
    let result = conn
        .interact(move |conn| {
            if !db::permissions::has_permission_on_project(
                conn,
                user.team_id,
                &user.roles,
                project_id,
                ProjectPermission::StorageLocationWrite,
            )? {
                return Err(Error::MissingPermission(Permission::StorageLocationWrite));
            }

            let output = diesel::insert_into(dsl::storage_locations)
                .values(&value)
                .returning(StorageLocationOutput::as_select())
                .get_result::<StorageLocationOutput>(conn)?;

            Ok::<_, crate::Error>(output)
        })
        .await??;

    Ok((StatusCode::ACCEPTED, Json(result)))
}

async fn get_global_location(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    Path(location_id): Path<StorageLocationId>,
) -> Result<impl IntoResponse, crate::Error> {
    get_location(state, user, None, location_id).await
}

async fn get_project_location(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    Path(path): Path<ProjectStorageLocationPath>,
) -> Result<impl IntoResponse, crate::Error> {
    get_location(state, user, Some(path.project_id), path.storage_location_id).await
}

async fn get_location(
    state: &State,
    user: UserInfo,
    project_id: Option<ProjectId>,
    location_id: StorageLocationId,
) -> Result<impl IntoResponse, crate::Error> {
    use db::storage_locations::dsl;
    let conn = state.db.get().await?;

    let (location, allowed) = conn
        .interact(move |conn| {
            dsl::storage_locations
                .select((
                    StorageLocationOutput::as_select(),
                    db::obj_allowed_or_projectless!(
                        user.team_id,
                        &user.roles,
                        dsl::project_id.assume_not_null(),
                        db::role_permissions::Permission::ProjectRead
                    ),
                ))
                .filter(dsl::storage_location_id.eq(location_id))
                .filter(dsl::project_id.is_not_distinct_from(project_id))
                .filter(dsl::team_id.eq(user.team_id))
                .first::<(StorageLocationOutput, bool)>(conn)
        })
        .await??;

    if !allowed {
        return Err(Error::MissingPermission(
            db::role_permissions::Permission::ProjectRead,
        ));
    }

    Ok((StatusCode::OK, Json(location)))
}

async fn disable_project_location(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    Path(path): Path<ProjectStorageLocationPath>,
) -> Result<impl IntoResponse, crate::Error> {
    disable_location(state, user, Some(path.project_id), path.storage_location_id).await
}

async fn disable_global_location(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
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
    use db::storage_locations::dsl;

    let conn = state.db.get().await?;
    conn.interact(move |conn| {
        must_have_permission_on_project(
            conn,
            &user,
            project_id.unwrap_or_else(ProjectId::nil),
            ProjectPermission::StorageLocationWrite,
        )?;

        diesel::update(dsl::storage_locations)
            .filter(dsl::storage_location_id.eq(location_id))
            .filter(dsl::project_id.is_not_distinct_from(project_id))
            .filter(dsl::team_id.eq(user.team_id))
            .set((dsl::deleted.eq(Some(Utc::now())),))
            .execute(conn)?;

        Ok::<(), Error>(())
    })
    .await??;

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
