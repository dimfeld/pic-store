use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use diesel::{sql_query, RunQueryDsl};
use serde::Serialize;

use crate::{shared_state::AppState, Error};

#[derive(Serialize)]
struct HealthResponse {
    /// If the database connection is ok
    database: bool,
    /// If all the other fields indicate healthy status.
    healthy: bool,
}

async fn check_db(state: &AppState) -> Result<(), Error> {
    let conn = state.db.get().await?;
    conn.interact(|conn| sql_query("SELECT 1").execute(conn))
        .await??;

    Ok(())
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let db_result = check_db(&state).await;

    (
        StatusCode::OK,
        Json(HealthResponse {
            healthy: db_result.is_ok(),
            database: db_result.is_ok(),
        }),
    )
}

pub fn configure() -> Router<AppState> {
    Router::new().route("/health", get(health))
}
