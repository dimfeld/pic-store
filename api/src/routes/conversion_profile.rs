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
use serde_json::json;

use db::conversion_profiles;
use db::{
    conversion_profiles::{ConversionProfile, NewConversionProfile},
    object_id::{ConversionProfileId, ProjectId},
};
use pic_store_db as db;

use crate::{auth::UserInfo, shared_state::State, Error};

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

async fn list_profiles(
    Extension(ref state): Extension<State>,
    Extension(ref user): Extension<UserInfo>,
) -> Result<impl IntoResponse, crate::Error> {
    let conn = state.db.get().await?;

    let team_id = user.team_id;
    let objects = conn
        .interact(move |conn| {
            // TODO PERM Extra checks for role permissions and such, once they exist, to reduce query load
            db::conversion_profiles::table
                .select(ConversionProfileOutput::as_select())
                .filter(db::conversion_profiles::team_id.eq(team_id))
                .load::<ConversionProfileOutput>(conn)
        })
        .await??;

    let objects = objects
        .into_iter()
        .map(ConversionProfileOutput::from)
        .collect::<Vec<_>>();

    Ok((StatusCode::OK, Json(objects)))
}

async fn write_profile(
    Extension(ref state): Extension<State>,
    Extension(ref user): Extension<UserInfo>,
    Path(profile_id): Path<ConversionProfileId>,
    Json(body): Json<ConversionProfileInput>,
) -> Result<impl IntoResponse, Error> {
    use db::conversion_profiles::dsl;

    let conn = state.db.get().await?;
    let result = conn
        .interact(move |conn| {
            // TODO PERM Permission checks
            diesel::update(db::conversion_profiles::table)
                .filter(db::conversion_profiles::conversion_profile_id.eq(profile_id))
                .set((dsl::name.eq(body.name), dsl::updated.eq(Utc::now())))
                .returning(ConversionProfileOutput::as_select())
                .get_result::<ConversionProfileOutput>(conn)
        })
        .await??;

    Ok((StatusCode::OK, Json(result)))
}

async fn new_profile(
    Extension(ref state): Extension<State>,
    Json(body): Json<ConversionProfileInput>,
) -> Result<impl IntoResponse, crate::Error> {
    use db::conversion_profiles::dsl;

    let value = NewConversionProfile {
        conversion_profile_id: ConversionProfileId::new(),
        name: body.name,
        team_id: state.team_id,
        project_id: body.project_id,
    };

    let conn = state.db.get().await?;
    let result = conn
        .interact(move |conn| {
            // TODO PERM
            diesel::insert_into(dsl::conversion_profiles)
                .values(&value)
                .returning(ConversionProfileOutput::as_select())
                .get_result::<ConversionProfileOutput>(conn)
        })
        .await??;

    Ok((StatusCode::ACCEPTED, Json(result)))
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
