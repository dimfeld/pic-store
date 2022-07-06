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
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use db::conversion_profiles::{ConversionProfile, NewConversionProfile};
use pic_store_auth::{
    AuthInfo, BiscuitExtractorError, BiscuitInfoExtractor, CheckBiscuitLayer, Fact, RequireBiscuit,
    UserAndTeamIds,
};
use pic_store_db as db;

use crate::{shared_state::State, Error};

#[derive(Debug, Deserialize)]
pub struct ConversionProfileInput {
    pub name: String,
    pub project_id: Option<Uuid>,
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
    id: Uuid,
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

#[derive(Clone)]
struct CheckProfileId {}

#[async_trait]
impl BiscuitInfoExtractor for CheckProfileId {
    type Object = db::conversion_profiles::ConversionProfile;

    async fn extract(
        &self,
        req: &mut RequestParts<Body>,
        user: &UserAndTeamIds,
    ) -> Result<(AuthInfo, ConversionProfile), BiscuitExtractorError> {
        let Path(profile_id) = Path::<Uuid>::from_request(req)
            .await
            .map_err(BiscuitExtractorError::internal_error)?;

        let conn = &req
            .extensions()
            .get::<State>()
            .unwrap()
            .db
            .get()
            .await
            .map_err(BiscuitExtractorError::internal_error)?;

        let team_id = user.team_id;
        let conversion_profile = conn
            .interact(move |conn| {
                db::conversion_profiles::table
                    .filter(db::conversion_profiles::conversion_profile_id.eq(profile_id))
                    .filter(db::conversion_profiles::team_id.eq(team_id))
                    .first::<ConversionProfile>(conn)
            })
            .await?
            .map_err(BiscuitExtractorError::internal_error)?;

        let auth_info = AuthInfo {
            resource_type: "conversion_profile",
            resource_id: conversion_profile.conversion_profile_id,
            team_id: conversion_profile.team_id,
            project_id: conversion_profile.project_id,
            deleted: conversion_profile.deleted.is_some(),
            operation: None,
        };

        Ok((auth_info, conversion_profile))
    }
}

async fn list_profiles() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn write_profile(
    Path(profile_id): Path<Uuid>,
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
    Extension(ref user): Extension<UserAndTeamIds>,
    Json(body): Json<ConversionProfileInput>,
) -> Result<impl IntoResponse, crate::Error> {
    use db::conversion_profiles::dsl;

    let value = NewConversionProfile {
        conversion_profile_id: Uuid::new_v4(),
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

async fn get_profile(Path(profile_id): Path<Uuid>) -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED, profile_id.to_string())
}

async fn disable_profile() -> impl IntoResponse {
    todo!();
    StatusCode::NOT_IMPLEMENTED
}

pub fn configure() -> Router {
    let item_routes = Router::new()
        .route("/:profile_id", get(get_profile))
        .route("/:profile_id", put(write_profile))
        .route("/:profile_id", delete(disable_profile))
        .route_layer(CheckBiscuitLayer::new(CheckProfileId {}));

    let routes = Router::new()
        .route("/", get(list_profiles))
        .route("/", post(new_profile))
        .merge(item_routes);

    Router::new().nest("/conversion_profiles", routes)
}
