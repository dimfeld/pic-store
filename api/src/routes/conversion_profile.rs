use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{FromRequest, Path, RequestParts},
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{NotSet, Set},
    ConnectionTrait, DatabaseConnection, EntityTrait, Statement, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;

use pic_store_auth::{
    AuthEvaluator, BiscuitExtractorError, BiscuitInfoExtractor, CheckBiscuitLayer, Fact,
    RequireBiscuit,
};
use pic_store_db as db;

use crate::{shared_state::State, Error};

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

#[derive(Debug, Serialize)]
pub struct ConversionProfileOutput {
    id: Uuid,
    name: String,
    updated: OffsetDateTime,
}

#[derive(Clone)]
struct CheckProfileId {}

#[async_trait]
impl BiscuitInfoExtractor for CheckProfileId {
    async fn extract(
        &self,
        mut req: RequestParts<Body>,
        auth: &mut AuthEvaluator<'_>,
    ) -> Result<RequestParts<Body>, BiscuitExtractorError> {
        let Path(profile_id) = Path::from_request(&mut req)
            .await
            .map_err(BiscuitExtractorError::internal_error)?;

        let db = &req.extensions().get::<State>().unwrap().db;

        let existing_profile = db::conversion_profile::Entity::find_by_id(profile_id)
            .one(db)
            .await
            .map_err(BiscuitExtractorError::internal_error)?
            .ok_or_else(|| Error::NotFound.response_tuple())?;

        auth.add_fact(Fact::Resource.with_value(profile_id))?;
        auth.set_operation_from_method(req.method())?;

        // There isn't yet any real permissions model, so just make sure that the team matches.
        auth.add_check(
            Fact::Team
                .check_if(existing_profile.team_id.to_string().as_str())
                .as_str(),
        )?;
        auth.allow()?;

        req.extensions_mut()
            .insert::<db::conversion_profile::Model>(existing_profile);

        Ok(req)
    }
}

async fn list_profiles() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn write_profile(
    Path(profile_id): Path<Uuid>,
    Extension(ref state): Extension<State>,
    Extension(profile): Extension<db::conversion_profile::Model>,
    Json(body): Json<ConversionProfileInput>,
) -> Result<impl IntoResponse, Error> {
    let now = sea_orm::prelude::TimeDateTimeWithTimeZone::now_utc();

    let mut item = db::conversion_profile::ActiveModel::from(profile);

    item.updated = Set(now);
    item.name = Set(body.name);

    let result = item.update(&state.db).await?;

    Ok((
        StatusCode::OK,
        Json(ConversionProfileOutput {
            id: result.id,
            name: result.name,
            updated: result.updated,
        }),
    ))
}

async fn new_profile(
    Extension(ref state): Extension<State>,
    biscuit: RequireBiscuit,
    Json(body): Json<ConversionProfileInput>,
) -> Result<impl IntoResponse, crate::Error> {
    let mut auth = state.auth.with_biscuit(&biscuit)?;
    let user_info = auth.get_user_and_team()?;

    let now = sea_orm::prelude::TimeDateTimeWithTimeZone::now_utc();
    let profile_id = Uuid::new_v4();

    let item = db::conversion_profile::ActiveModel {
        id: Set(profile_id),
        name: Set(body.name),
        updated: Set(now),
        team_id: Set(user_info.team_id),
        ..Default::default()
    };

    item.insert(&state.db).await?;

    Ok((StatusCode::ACCEPTED, Json(json!({}))))
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
