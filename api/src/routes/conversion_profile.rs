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
    ConnectionTrait, DatabaseConnection, Statement, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use pic_store_auth::RequireBiscuit;
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

async fn write_profile() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn new_profile(
    Extension(ref state): Extension<State>,
    biscuit: RequireBiscuit,
    Json(body): Json<ConversionProfileInput>,
) -> Result<impl IntoResponse, crate::Error> {
    let now = sea_orm::prelude::TimeDateTimeWithTimeZone::now_utc();
    let profile_id = Uuid::new_v4();

    let (team_id, user_id): (String, String) = biscuit
        .authorizer()?
        .query(r##"data($team, $user) <- team($team), user($user)"##)?
        .pop()
        .ok_or(Error::Unauthorized)?;

    let item = db::conversion_profile::ActiveModel {
        id: Set(profile_id),
        name: Set(body.name),
        updated: Set(now),
        team_id: Set(team_id),
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
