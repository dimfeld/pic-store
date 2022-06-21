use axum::{
    extract::Path,
    http::StatusCode,
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
use uuid::Uuid;

use pic_store_auth::{Fact, RequireBiscuit};
use pic_store_db as db;

use crate::{shared_state::State, Error};

#[derive(Debug, Serialize, Deserialize)]
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

async fn list_profiles() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn write_profile(
    Path(profile_id): Path<Uuid>,
    Extension(ref state): Extension<State>,
    biscuit: RequireBiscuit,
    Json(body): Json<ConversionProfileInput>,
) -> Result<impl IntoResponse, Error> {
    let mut auth = state.auth.with_biscuit(&biscuit)?;

    let existing_profile = db::conversion_profile::Entity::find_by_id(profile_id)
        .one(&state.db)
        .await?
        .ok_or(Error::NotFound)?;

    auth.add_fact(Fact::Resource.with_value(profile_id))?;
    auth.add_fact(Fact::Operation.with_value("write"))?;

    // There isn't yet any real permissions model, so just make sure that the team matches.
    auth.add_check(
        Fact::Team
            .check_if(existing_profile.team_id.to_string().as_str())
            .as_str(),
    )?;
    auth.allow()?;

    auth.authorize()?;

    let user_info = auth.get_user_and_team()?;

    let now = sea_orm::prelude::TimeDateTimeWithTimeZone::now_utc();

    let mut item = db::conversion_profile::ActiveModel::from(existing_profile);

    item.updated = Set(now);
    item.name = Set(body.name);

    let result = item.update(&state.db).await?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "id": result.id,
            "name": result.name,
            "updated": result.updated.to_string(),
        })),
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
    let routes = Router::new()
        .route("/", get(list_profiles))
        .route("/", post(new_profile))
        .route("/:profile_id", get(get_profile))
        .route("/:profile_id", put(write_profile))
        .route("/:profile_id", delete(disable_profile));

    Router::new().nest("/conversion_profiles", routes)
}
