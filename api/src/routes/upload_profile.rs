use axum::{
    extract::{BodyStream, ContentLengthLimit, Multipart, Path},
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use db::{
    object_id::{ConversionProfileId, ProjectId, StorageLocationId, TeamId, UploadProfileId},
    permissions::ProjectPermission,
    upload_profiles::{self, NewUploadProfile},
    Permission, PoolExt,
};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use pic_store_db as db;
use pic_store_storage as storage;

use crate::{
    auth::{must_have_permission_on_project, UserInfo},
    shared_state::State,
    Error, Result,
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
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    Path(project_id): Path<ProjectId>,
) -> Result<impl IntoResponse> {
    use db::upload_profiles::dsl;
    let objects = state
        .db
        .interact(move |conn| {
            dsl::upload_profiles
                .select(UploadProfileOutput::as_select())
                .filter(dsl::deleted.is_null())
                .filter(db::obj_allowed!(
                    user.team_id,
                    &user.roles,
                    dsl::project_id,
                    Permission::ProjectRead
                ))
                .filter(dsl::project_id.eq(project_id))
                .load::<UploadProfileOutput>(conn)
                .map_err(Error::from)
        })
        .await?;

    Ok((StatusCode::OK, Json(objects)))
}

async fn get_project_upload_profile(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    Path((project_id, profile_id)): Path<(ProjectId, UploadProfileId)>,
) -> Result<impl IntoResponse> {
    use db::upload_profiles::dsl;
    let (object, allowed) = state
        .db
        .interact(move |conn| {
            dsl::upload_profiles
                .select((
                    UploadProfileOutput::as_select(),
                    db::obj_allowed!(
                        user.team_id,
                        &user.roles,
                        dsl::project_id.assume_not_null(),
                        db::role_permissions::Permission::ProjectRead
                    ),
                ))
                .filter(dsl::id.eq(profile_id))
                .filter(dsl::team_id.eq(user.team_id))
                .filter(dsl::project_id.eq(project_id))
                .first::<(UploadProfileOutput, bool)>(conn)
                .map_err(Error::from)
        })
        .await?;

    if !allowed {
        return Err(Error::MissingPermission(
            db::role_permissions::Permission::ProjectRead,
        ));
    }

    Ok((StatusCode::OK, Json(object)))
}

async fn write_project_upload_profile(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    Path((project_id, profile_id)): Path<(ProjectId, UploadProfileId)>,
    Json(body): Json<UploadProfileInput>,
) -> Result<impl IntoResponse> {
    use db::upload_profiles::dsl;

    let result = state
        .db
        .interact(move |conn| {
            must_have_permission_on_project(
                conn,
                &user,
                project_id,
                ProjectPermission::ProjectWrite,
            )?;

            diesel::update(dsl::upload_profiles)
                .filter(dsl::id.eq(profile_id))
                .filter(dsl::project_id.eq(project_id))
                .filter(dsl::team_id.eq(user.team_id))
                .set((dsl::name.eq(body.name), dsl::updated.eq(Utc::now())))
                .returning(UploadProfileOutput::as_select())
                .get_result::<UploadProfileOutput>(conn)
                .map_err(Error::from)
        })
        .await?;

    Ok((StatusCode::OK, Json(result)))
}

async fn new_project_upload_profile(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
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

    let result = state
        .db
        .interact(move |conn| {
            must_have_permission_on_project(
                conn,
                &user,
                project_id,
                ProjectPermission::ProjectWrite,
            )?;

            diesel::insert_into(dsl::upload_profiles)
                .values(&value)
                .returning(UploadProfileOutput::as_select())
                .get_result::<UploadProfileOutput>(conn)
                .map_err(Error::from)
        })
        .await?;

    Ok((StatusCode::OK, Json(result)))
}

async fn disable_project_upload_profile(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    Path((project_id, profile_id)): Path<(ProjectId, UploadProfileId)>,
) -> Result<impl IntoResponse> {
    use db::upload_profiles::dsl;

    state
        .db
        .interact(move |conn| {
            must_have_permission_on_project(
                conn,
                &user,
                project_id,
                ProjectPermission::ProjectWrite,
            )?;

            diesel::update(dsl::upload_profiles)
                .filter(dsl::id.eq(profile_id))
                .filter(dsl::project_id.eq(project_id))
                .filter(dsl::team_id.eq(user.team_id))
                .set(dsl::deleted.eq(Some(Utc::now())))
                .execute(conn)
                .map_err(Error::from)
        })
        .await?;

    Ok((StatusCode::OK, Json(json!({}))))
}

pub fn configure() -> Router {
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
