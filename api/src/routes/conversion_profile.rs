use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{FromRequest, Path, RequestParts},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use http::Method;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use db::{
    conversion_profiles::{ConversionProfile, NewConversionProfile},
    object_id::{ConversionProfileId, ProjectId},
};
use pic_store_db as db;

use crate::{shared_state::State, Error};

#[derive(Debug, Deserialize)]
pub struct ConversionProfileInput {
    pub name: String,
    pub project_id: Option<ProjectId>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversionProfileItemInput {
    pub name: String,
    pub format: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ConversionProfileOutput {
    id: ConversionProfileId,
    name: String,
    updated: DateTime<Utc>,
}

impl From<ConversionProfile> for ConversionProfileOutput {
    fn from(value: ConversionProfile) -> Self {
        ConversionProfileOutput {
            id: value.conversion_profile_id,
            name: value.name,
            updated: value.updated,
        }
    }
}

async fn list_profiles(
    Extension(ref state): Extension<State>,
) -> Result<impl IntoResponse, crate::Error> {
    let conn = state.db.get().await?;

    let objects = conn
        .interact(move |conn| {
            // TODO PERM Extra checks for role permissions and such, once they exist, to reduce query load
            db::conversion_profiles::table
                .filter(db::conversion_profiles::team_id.eq(user.team_id))
                .load::<ConversionProfile>(conn)
        })
        .await??;

    let objects = objects
        .into_iter()
        .filter_map(|o| {
            let mut auth = auth.clone();
            auth.set_resource(o.conversion_profile_id.to_string())
                .ok()?;
            auth.set_deleted(o.deleted.is_some()).ok()?;

            if let Some(project_id) = o.project_id.as_ref() {
                auth.set_project(project_id).ok()?;
            }

            auth.authorize()
                .ok()
                .map(|_| ConversionProfileOutput::from(o))
        })
        .collect::<Vec<_>>();

    Ok((StatusCode::OK, Json(objects)))
}

async fn write_profile(
    Extension(ref state): Extension<State>,
    Extension(profile): Extension<ConversionProfile>,
    Json(body): Json<ConversionProfileInput>,
) -> Result<impl IntoResponse, Error> {
    use db::conversion_profiles::dsl;

    let conn = state.db.get().await?;
    let result = conn
        .interact(move |conn| {
            diesel::update(&profile)
                .set((dsl::name.eq(body.name), dsl::updated.eq(Utc::now())))
                .get_result::<ConversionProfile>(conn)
        })
        .await??;

    Ok((StatusCode::OK, Json(ConversionProfileOutput::from(result))))
}

async fn new_profile(
    Extension(ref state): Extension<State>,
    Json(body): Json<ConversionProfileInput>,
) -> Result<impl IntoResponse, crate::Error> {
    use db::conversion_profiles::dsl;

    let value = NewConversionProfile {
        conversion_profile_id: ConversionProfileId::new(),
        name: body.name,
        team_id: user.team_id,
        project_id: body.project_id,
    };

    let conn = state.db.get().await?;
    let result = conn
        .interact(move |conn| {
            diesel::insert_into(dsl::conversion_profiles)
                .values(&value)
                .get_result::<ConversionProfile>(conn)
        })
        .await??;

    Ok((
        StatusCode::ACCEPTED,
        Json(ConversionProfileOutput::from(result)),
    ))
}

async fn get_profile(Extension(profile): Extension<ConversionProfile>) -> impl IntoResponse {
    (StatusCode::OK, Json(ConversionProfileOutput::from(profile)))
}

async fn disable_profile(
    Extension(ref state): Extension<State>,
    Extension(profile): Extension<ConversionProfile>,
) -> Result<impl IntoResponse, crate::Error> {
    use db::conversion_profiles::dsl;

    let conn = state.db.get().await?;
    conn.interact(move |conn| {
        diesel::update(&profile)
            .set((dsl::deleted.eq(Some(Utc::now())),))
            .execute(conn)
    })
    .await??;

    Ok((StatusCode::OK, Json(json!({}))))
}

pub fn configure() -> Router {
    let item_routes = Router::new()
        .route("/:profile_id", get(get_profile))
        .route("/:profile_id", put(write_profile))
        .route("/:profile_id", delete(disable_profile));

    let routes = Router::new()
        .route("/", get(list_profiles))
        .route("/", post(new_profile))
        .merge(item_routes);

    Router::new().nest("/conversion_profiles", routes)
}
