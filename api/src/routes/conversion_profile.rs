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
use serde_json::json;

use db::{
    conversion_profiles,
    conversion_profiles::{ConversionProfile, NewConversionProfile},
    object_id::{ConversionProfileId, ProjectId},
    permissions::ProjectPermission,
    Permission,
};
use pic_store_db as db;

use crate::{
    auth::{must_have_permission_on_project, UserInfo},
    shared_state::State,
    Error,
};

#[derive(Debug, Deserialize)]
pub struct ConversionProfileInput {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversionProfileItemInput {
    pub name: String,
    pub format: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Serialize, Queryable, Selectable)]
#[diesel(table_name = conversion_profiles)]
pub struct ConversionProfileOutput {
    #[serde(rename = "id")]
    conversion_profile_id: ConversionProfileId,
    name: String,
    updated: DateTime<Utc>,
}

impl From<ConversionProfile> for ConversionProfileOutput {
    fn from(value: ConversionProfile) -> Self {
        ConversionProfileOutput {
            conversion_profile_id: value.conversion_profile_id,
            name: value.name,
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
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    Path(project_id): Path<ProjectId>,
) -> Result<impl IntoResponse, crate::Error> {
    list_profiles(state, user, Some(project_id)).await
}

async fn list_global_profiles(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
) -> Result<impl IntoResponse, Error> {
    list_profiles(state, user, None).await
}

async fn list_profiles(
    state: &State,
    user: UserInfo,
    project_id: Option<ProjectId>,
) -> Result<impl IntoResponse, Error> {
    use db::conversion_profiles::dsl;
    let conn = state.db.get().await?;

    let objects = conn
        .interact(move |conn| {
            let q = dsl::conversion_profiles
                .select(ConversionProfileOutput::as_select())
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

            q.load::<ConversionProfileOutput>(conn)
        })
        .await??;

    Ok((StatusCode::OK, Json(objects)))
}

async fn write_project_profile(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
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
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    Path(profile_id): Path<ConversionProfileId>,
    Json(body): Json<ConversionProfileInput>,
) -> Result<impl IntoResponse, Error> {
    write_profile(state, user, None, profile_id, body).await
}

async fn write_profile(
    state: &State,
    user: UserInfo,
    project_id: Option<ProjectId>,
    profile_id: ConversionProfileId,
    body: ConversionProfileInput,
) -> Result<impl IntoResponse, Error> {
    use db::conversion_profiles::dsl;

    let conn = state.db.get().await?;

    let result = conn
        .interact(move |conn| {
            must_have_permission_on_project(
                conn,
                &user,
                project_id.unwrap_or_else(ProjectId::nil),
                ProjectPermission::ConversionProfileWrite,
            )?;

            let result = diesel::update(dsl::conversion_profiles)
                .filter(dsl::conversion_profile_id.eq(profile_id))
                .filter(dsl::project_id.is_not_distinct_from(project_id))
                .filter(dsl::team_id.eq(user.team_id))
                .set((dsl::name.eq(body.name), dsl::updated.eq(Utc::now())))
                .returning(ConversionProfileOutput::as_select())
                .get_result::<ConversionProfileOutput>(conn)?;

            Ok::<_, Error>(result)
        })
        .await??;

    Ok((StatusCode::OK, Json(result)))
}

async fn new_project_profile(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    Path(project_id): Path<ProjectId>,
    Json(body): Json<ConversionProfileInput>,
) -> Result<impl IntoResponse, crate::Error> {
    new_profile(state, user, Some(project_id), body).await
}

async fn new_global_profile(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    Json(body): Json<ConversionProfileInput>,
) -> Result<impl IntoResponse, crate::Error> {
    new_profile(state, user, None, body).await
}

async fn new_profile(
    state: &State,
    user: UserInfo,
    project_id: Option<ProjectId>,
    body: ConversionProfileInput,
) -> Result<impl IntoResponse, Error> {
    use db::conversion_profiles::dsl;

    let value = NewConversionProfile {
        conversion_profile_id: ConversionProfileId::new(),
        name: body.name,
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
                ProjectPermission::ConversionProfileWrite,
            )? {
                return Err(Error::Unauthorized);
            }

            let output = diesel::insert_into(dsl::conversion_profiles)
                .values(&value)
                .returning(ConversionProfileOutput::as_select())
                .get_result::<ConversionProfileOutput>(conn)?;

            Ok::<_, crate::Error>(output)
        })
        .await??;

    Ok((StatusCode::ACCEPTED, Json(result)))
}

async fn get_global_profile(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    Path(profile_id): Path<ConversionProfileId>,
) -> Result<impl IntoResponse, crate::Error> {
    get_profile(state, user, None, profile_id).await
}

async fn get_project_profile(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    Path(path): Path<ProjectConversionProfilePath>,
) -> Result<impl IntoResponse, crate::Error> {
    get_profile(
        state,
        user,
        Some(path.project_id),
        path.conversion_profile_id,
    )
    .await
}

async fn get_profile(
    state: &State,
    user: UserInfo,
    project_id: Option<ProjectId>,
    profile_id: ConversionProfileId,
) -> Result<impl IntoResponse, crate::Error> {
    use db::conversion_profiles::dsl;
    let conn = state.db.get().await?;

    let (profile, allowed) = conn
        .interact(move |conn| {
            dsl::conversion_profiles
                .select((
                    ConversionProfileOutput::as_select(),
                    db::obj_allowed_or_projectless!(
                        user.team_id,
                        &user.roles,
                        dsl::project_id.assume_not_null(),
                        db::role_permissions::Permission::ProjectRead
                    ),
                ))
                .filter(dsl::conversion_profile_id.eq(profile_id))
                .filter(dsl::project_id.is_not_distinct_from(project_id))
                .filter(dsl::team_id.eq(user.team_id))
                .first::<(ConversionProfileOutput, bool)>(conn)
        })
        .await??;

    if !allowed {
        return Err(Error::Unauthorized);
    }

    Ok((StatusCode::OK, Json(profile)))
}

async fn disable_project_profile(
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
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
    Extension(ref state): Extension<State>,
    Extension(user): Extension<UserInfo>,
    Path(profile_id): Path<ConversionProfileId>,
) -> Result<impl IntoResponse, crate::Error> {
    disable_profile(state, user, None, profile_id).await
}

async fn disable_profile(
    state: &State,
    user: UserInfo,
    project_id: Option<ProjectId>,
    profile_id: ConversionProfileId,
) -> Result<impl IntoResponse, crate::Error> {
    use db::conversion_profiles::dsl;

    let conn = state.db.get().await?;
    conn.interact(move |conn| {
        must_have_permission_on_project(
            conn,
            &user,
            project_id.unwrap_or_else(ProjectId::nil),
            ProjectPermission::ConversionProfileWrite,
        )?;

        diesel::update(dsl::conversion_profiles)
            .filter(dsl::conversion_profile_id.eq(profile_id))
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
        .route("/", get(list_project_profiles))
        .route("/", post(new_project_profile))
        .route("/:profile_id", get(get_project_profile))
        .route("/:profile_id", put(write_project_profile))
        .route("/:profile_id", delete(disable_project_profile));

    let project_router =
        Router::new().nest("/projects/:project_id/conversion_profiles", project_routes);

    let global_routes = Router::new()
        .route("/", get(list_global_profiles))
        .route("/", post(new_global_profile))
        .route("/:profile_id", get(get_global_profile))
        .route("/:profile_id", put(write_global_profile))
        .route("/:profile_id", delete(disable_global_profile));

    let global_router = Router::new().nest("/projects/global/conversion_profiles", global_routes);

    global_router.merge(project_router)
}
